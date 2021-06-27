// standard imports
use std::cmp::{max, min};
use std::collections::HashMap;
use std::convert::TryInto;
use std::fs::File;
use std::io::{Cursor, Read};
use std::rc::Rc;
use std::sync::Arc;

// Vulkano imports
use vulkano::buffer::{BufferUsage, ImmutableBuffer, TypedBufferAccess};
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, DynamicState, PrimaryAutoCommandBuffer,
    SubpassContents,
};
use vulkano::Handle;

use vulkano::descriptor::descriptor_set::{
    PersistentDescriptorSet, PersistentDescriptorSetBuilder, PersistentDescriptorSetImg,
    PersistentDescriptorSetSampler,
};
use vulkano::device::{Device, DeviceExtensions, Queue};
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::image::{ImageDimensions, ImageUsage, ImmutableImage, MipmapsCount, SwapchainImage};
use vulkano::instance::{Instance, InstanceExtensions, PhysicalDevice};
use vulkano::memory::DeviceMemoryAllocError;
use vulkano::pipeline::vertex::SingleBufferDefinition;
use vulkano::pipeline::viewport::Viewport;
use vulkano::pipeline::{GraphicsPipeline, GraphicsPipelineAbstract};
use vulkano::render_pass::RenderPass;
use vulkano::render_pass::{Framebuffer, FramebufferAbstract, Subpass};
use vulkano::sampler::{Filter, MipmapMode, Sampler, SamplerAddressMode};
use vulkano::swapchain;
use vulkano::swapchain::{AcquireError, Surface, Swapchain, SwapchainCreationError};
use vulkano::sync;
use vulkano::sync::{FlushError, GpuFuture};
use vulkano::Version;
use vulkano::VulkanObject;

// SDL2 imports
use sdl2::video::{Window, WindowContext};

// other imports
use super::sendable::Sendable;
use png;

/// Use of a macro due to literals needed.
/// This creates a new pipeline object (using the specified shaders) and appends it to the HashMap.
#[macro_use]
macro_rules! create_pipeline {
    ($name: expr, $device: expr, $render_pass: expr, $vs_path: expr, $fs_path: expr, $map: expr) => {{
        mod vertex_shader {
            vulkano_shaders::shader! {
               ty: "vertex",
               path: $vs_path
            }
        }

        mod fragment_shader {
            vulkano_shaders::shader! {
                ty: "fragment",
                path: $fs_path
            }
        }

        let vert_shader = vertex_shader::Shader::load($device.clone()).expect(&format!(
            "Couldn't load Vertex Shader: pipeline name: {},\nshader path: {}",
            $name, $vs_path
        ));
        let frag_shader = fragment_shader::Shader::load($device.clone()).expect(&format!(
            "Couldn't load Fragment Shader: pipeline name: {},\nshader path: {}",
            $name, $fs_path
        ));

        let pipeline = Arc::new(
            GraphicsPipeline::start()
                .vertex_input_single_buffer::<Vertex>()
                .vertex_shader(vert_shader.main_entry_point(), ())
                .triangle_strip()
                .viewports_dynamic_scissors_irrelevant(1)
                .blend_alpha_blending()
                .fragment_shader(frag_shader.main_entry_point(), ())
                .render_pass(Subpass::from($render_pass.clone(), 0).unwrap())
                .build($device.clone())
                .expect("Couldn't create new Vulkan Graphics Pipeline"),
        );
        $map.insert($name.to_string(), pipeline.clone());
    };};
}

pub type Texture = Arc<ImageView<Arc<ImmutableImage>>>;
pub type ImageDescriptorSet = Arc<
    PersistentDescriptorSet<(
        (
            (),
            PersistentDescriptorSetImg<Arc<ImageView<Arc<ImmutableImage>>>>,
        ),
        PersistentDescriptorSetSampler,
    )>,
>;

/// Struct to handle connections to the Vulkano (and thus Vulkan) API
pub struct GraphicsHandler {
    instance: Arc<Instance>,
    swapchain: SwapchainHandler,
    render_pass: Arc<RenderPass>,
    pipelines: HashMap<String, Arc<GraphicsPipeline<SingleBufferDefinition<Vertex>>>>,
    previous_frame_end: Option<Box<dyn GpuFuture>>,
    device: Arc<Device>,
    queue: Arc<Queue>,
}

impl GraphicsHandler {
    /// Vulkan object handler instancing and init
    pub fn new(window: &Window) -> Self {
        let instance = create_instance();

        let surface = create_surface(instance.clone(), window);

        // Get the device info and queue
        let (physical, device, queue) = get_device(&instance, surface.clone());

        let (swapchain, images) =
            create_raw_swapchain(window, device.clone(), surface.clone(), physical);

        let render_pass = Arc::new(
            vulkano::single_pass_renderpass!(
                device.clone(),
                attachments: {
                    color: {
                        load: Clear,
                        store: Store,
                        format: swapchain.format(),
                        samples: 1,
                    }
                },
                pass: {
                    color: [color],
                    depth_stencil: {}
                }
            )
            .expect("Couldn't create new Vulkan RenderPass"),
        );

        let mut pipelines = HashMap::new();
        create_pipeline!(
            "SimpleTriangle",
            device.clone(),
            render_pass.clone(),
            "assets/shaders/triangle.vert",
            "assets/shaders/triangle.frag",
            &mut pipelines
        );
        create_pipeline!(
            "Sprite",
            device.clone(),
            render_pass.clone(),
            "assets/shaders/sprite.vert",
            "assets/shaders/sprite.frag",
            &mut pipelines
        );

        let swapchain = SwapchainHandler::new(swapchain.clone(), images, render_pass.clone());

        let previous_frame_end = Some(sync::now(device.clone()).boxed());
        Self {
            instance: instance.clone(),
            swapchain,
            render_pass: render_pass.clone(),
            pipelines,
            previous_frame_end,
            device,
            queue,
        }
    }

    pub fn vulkan_loop(&mut self, resized: bool, window: &Window) {
        {
            // If the window is being resized, return true, otherwise keep the original value (in case of pending resizes)
            let recreate: bool = {
                if resized {
                    true
                } else {
                    self.swapchain.get_recreate()
                }
            };

            self.swapchain.set_recreate(recreate);

            let pass = self.render_pass.clone();
            let swapchain = self.get_swapchain();

            // Not an actual error, just a way to signify the need to retry the procedure
            if let Err(_) = swapchain.check_and_recreate(window, pass) {
                return;
            }
        }
        // start of the actual loop code
        self.previous_frame_end.as_mut().unwrap().cleanup_finished();
        let (image_num, suboptimal, acquire_future) =
            match swapchain::acquire_next_image(self.get_swapchain().chain.clone(), None) {
                Ok(r) => r,
                Err(AcquireError::OutOfDate) => {
                    self.get_swapchain().set_recreate(true);
                    return;
                }
                Err(e) => panic!("Couldn't acquire next image from Vulkan Swapchain: {}", e),
            };
        self.get_swapchain().set_recreate(suboptimal);

        let mut builder = AutoCommandBufferBuilder::primary(
            self.get_device(),
            self.queue.family(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .expect("Couldn't build Vulkan AutoCommandBuffer");

        let vao = VertexArray::from(vec![
            Vertex {
                position: [-0.5, 0.5],
                color: [1.0, 0.0, 0.0],
            },
            Vertex {
                position: [-0.5, -0.5],
                color: [0.0, 0.0, 0.0],
            },
            Vertex {
                position: [0.5, -0.5],
                color: [0.0, 0.0, 1.0],
            },
            Vertex {
                position: [0.5, 0.5],
                color: [0.0, 0.0, 0.0],
            },
        ]);
        let indices = self.new_index_buffer(&[0, 1, 2, 2, 3, 0]);
        let shape = PrimitiveShape {
            vertex_buffer: self.new_vertex_buffer(vao, indices),
        };

        let sprite = Sprite::new("assets/rust.png", self);

        builder
            .begin_render_pass(
                self.get_swapchain().framebuffers[image_num].clone(),
                SubpassContents::Inline,
                vec![[0.0, 0.0, 0.0, 1.0].into()],
            )
            .expect("Couldn't begin Vulkan Render Pass");
        shape.draw(self, &mut builder);
        sprite.draw(self, &mut builder);

        builder
            .end_render_pass()
            .expect("Couldn't properly end Vulkan Render Pass");

        let command_buffer = builder
            .build()
            .expect("Couldn't build Vulkan Command Buffer");

        let future = self
            .previous_frame_end
            .take()
            .unwrap()
            .join(acquire_future)
            .then_execute(self.queue.clone(), command_buffer)
            .expect("Couldn't execute Vulkan Command Buffer")
            .then_swapchain_present(
                self.queue.clone(),
                self.get_swapchain().chain.clone(),
                image_num,
            )
            .then_signal_fence_and_flush();
        match future {
            Ok(future) => {
                self.previous_frame_end = Some(future.boxed());
            }
            Err(FlushError::OutOfDate) => {
                self.get_swapchain().set_recreate(true);
                self.previous_frame_end = Some(sync::now(self.get_device()).boxed());
            }
            Err(e) => {
                eprintln!("Failed to flush Vulkan Future: {:?}", e);
                self.previous_frame_end = Some(sync::now(self.get_device()).boxed());
            }
        }
    }

    fn get_swapchain(&mut self) -> &mut SwapchainHandler {
        &mut self.swapchain
    }

    fn get_device(&self) -> Arc<Device> {
        self.device.clone()
    }

    fn get_pipeline(&self, name: &str) -> Arc<GraphicsPipeline<SingleBufferDefinition<Vertex>>> {
        self.pipelines
            .get(name)
            .expect("No Vulkan Pipeline under this name was found")
            .clone()
    }

    fn get_queue(&self) -> Arc<Queue> {
        self.queue.clone()
    }

    fn new_vertex_buffer(&self, vao: VertexArray, indices: Arc<dyn TypedBufferAccess<Content = [u16]> + Send + Sync>) -> VertexBuffer {
        VertexBuffer::new(self, vao, indices)
            .expect("Device Memory Allocation Error during creation of new Vertex Buffer")
    }

    fn new_index_buffer(
        &self,
        indices: &[u16],
    ) -> Arc<dyn TypedBufferAccess<Content = [u16]> + Send + Sync> {
        let (buffer, future) = ImmutableBuffer::from_iter(
            indices.iter().cloned(),
            BufferUsage::index_buffer(),
            self.queue.clone(),
        )
        .unwrap();
        future.flush().unwrap();
        buffer
    }

    fn create_empty_descriptor_set_builder(
        &self,
        pipeline_name: &str,
        layout_number: usize,
    ) -> PersistentDescriptorSetBuilder<()> {
        let pipeline = self.get_pipeline(pipeline_name);
        let layout = pipeline
            .layout()
            .descriptor_set_layout(layout_number)
            .expect("Couldn't use Descriptor Set Layout");
        PersistentDescriptorSet::start(layout.clone())
    }

    fn create_and_bind_texture<R>(
        &self,
        texture_path: &str,
        desc_set_builder: PersistentDescriptorSetBuilder<R>,
        sampler: Arc<Sampler>,
    ) -> PersistentDescriptorSetBuilder<(
        (
            R,
            PersistentDescriptorSetImg<Arc<ImageView<Arc<ImmutableImage>>>>,
        ),
        PersistentDescriptorSetSampler,
    )> {
        let (texture, _tex_future) = {
            let decoder = png::Decoder::new(File::open(texture_path).unwrap());
            let (info, mut reader) = decoder.read_info().unwrap();

            let mut buf = vec![0; info.buffer_size()];

            reader.next_frame(&mut buf).unwrap();

            let dimensions = ImageDimensions::Dim2d{ width: info.width, height: info.height, array_layers: 1 };
                
            let (image, future) = ImmutableImage::from_iter(
                buf.iter().cloned(),
                dimensions,
                MipmapsCount::One,
                Format::R8G8B8A8Srgb,
                self.get_queue(),
            )
            .unwrap();
            (ImageView::new(image).unwrap(), future)
        };

        desc_set_builder
            .add_sampled_image(texture, sampler)
            .expect("Couldn't add Sampled Image to Descriptor Set")
    }

    fn create_texture_sampler(&self) -> Arc<Sampler> {
        Sampler::new(
            self.get_device(),
            Filter::Linear,
            Filter::Linear,
            MipmapMode::Nearest,
            SamplerAddressMode::Repeat,
            SamplerAddressMode::Repeat,
            SamplerAddressMode::Repeat,
            0.0,
            1.0,
            0.0,
            0.0,
        )
        .expect("Couldn't create Vulkan Texture Sampler")
    }
}

/// Type to hold swapchain and corresponding images
struct SwapchainHandler {
    chain: Arc<Swapchain<Sendable<Rc<WindowContext>>>>,
    images: Vec<Arc<SwapchainImage<Sendable<Rc<WindowContext>>>>>,
    framebuffers: Vec<Arc<dyn FramebufferAbstract + Send + Sync>>,
    must_recreate: bool,
    dynamic_state: DynamicState,
}

impl SwapchainHandler {
    fn new(
        swapchain: Arc<Swapchain<Sendable<Rc<WindowContext>>>>,
        images: Vec<Arc<SwapchainImage<Sendable<Rc<WindowContext>>>>>,
        render_pass: Arc<RenderPass>,
    ) -> Self {
        let mut dynamic_state = DynamicState {
            line_width: None,
            viewports: None,
            scissors: None,
            compare_mask: None,
            write_mask: None,
            reference: None,
        };

        let framebuffers =
            window_size_dependent_setup(&images[..], render_pass.clone(), &mut dynamic_state);

        Self {
            chain: swapchain,
            images: images,
            framebuffers,
            must_recreate: false,
            dynamic_state,
        }
    }

    fn check_and_recreate(&mut self, window: &Window, pass: Arc<RenderPass>) -> Result<(), ()> {
        if self.must_recreate {
            let dimensions: [u32; 2] = {
                let size = window.size();
                [size.0, size.1]
            };

            let (new_swapchain, new_images) =
                match self.chain.recreate().dimensions(dimensions).build() {
                    Ok(r) => r,
                    Err(SwapchainCreationError::UnsupportedDimensions) => return Err(()),
                    Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
                };

            self.chain = new_swapchain;
            self.images = new_images;

            let framebuffers = window_size_dependent_setup(
                &self.images[..],
                pass.clone(),
                &mut self.dynamic_state,
            );
            self.framebuffers = framebuffers;
            self.must_recreate = false;
        }
        Ok(())
    }

    fn get_recreate(&self) -> bool {
        self.must_recreate
    }

    fn set_recreate(&mut self, new_value: bool) {
        self.must_recreate = new_value;
    }
}

/// Struct to hold vertex data
#[derive(Default, Copy, Clone)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 3],
}
vulkano::impl_vertex!(Vertex, position, color);

/// Simple struct to hold an array of vertices
struct VertexArray {
    data: Vec<Vertex>,
}

impl From<Vec<Vertex>> for VertexArray {
    fn from(vec: Vec<Vertex>) -> Self {
        Self { data: vec }
    }
}

/// Struct to hold a vertex buffer with data
struct VertexBuffer {
    buffer: Arc<ImmutableBuffer<[Vertex]>>,
    indices: Arc<dyn TypedBufferAccess<Content = [u16]> + Send + Sync>,
}

impl VertexBuffer {
    pub fn new(
        handler: &GraphicsHandler,
        array: VertexArray,
        indices: Arc<dyn TypedBufferAccess<Content = [u16]> + Send + Sync>,
    ) -> Result<Self, DeviceMemoryAllocError> {
        let (buffer, future) = ImmutableBuffer::from_iter(
            array.data.iter().cloned(),
            BufferUsage::vertex_buffer(),
            handler.queue.clone(),
        )
        .unwrap();

        future.flush().unwrap();

        Ok(Self { buffer, indices })
    }

    pub fn get_vertices(&self) -> Arc<ImmutableBuffer<[Vertex]>> {
        self.buffer.clone()
    }

    pub fn get_indices(&self) -> Arc<dyn TypedBufferAccess<Content = [u16]> + Send + Sync> {
        self.indices.clone()
    }
}

/// Called during init and at every resize of the window
/// There is no error handling, if something goes wrong here, panic is the best solution
fn window_size_dependent_setup(
    images: &[Arc<SwapchainImage<Sendable<Rc<WindowContext>>>>],
    render_pass: Arc<RenderPass>,
    dynamic_state: &mut DynamicState,
) -> Vec<Arc<dyn FramebufferAbstract + Send + Sync>> {
    let dimensions = images[0].dimensions();

    let viewport = Viewport {
        origin: [0.0, 0.0],
        dimensions: [dimensions[0] as f32, dimensions[1] as f32],
        depth_range: 0.0..1.0,
    };
    dynamic_state.viewports = Some(vec![viewport]);
    images
        .iter()
        .map(|image| {
            let view = ImageView::new(image.clone())
                .expect("Couldn't create Image View on window resize/init");
            Arc::new(
                Framebuffer::start(render_pass.clone())
                    .add(view)
                    .expect("Couldn't add Image View on Framebuffer creation")
                    .build()
                    .expect("Couldn't build Framebuffer on window resize"),
            ) as Arc<dyn FramebufferAbstract + Send + Sync>
        })
        .collect::<Vec<_>>()
}

fn create_instance() -> Arc<Instance> {
    let instance_extensions = InstanceExtensions::supported_by_core()
        .expect("Couldn't obtain Vulkan Instance Extensions");

    Instance::new(None, Version::V1_2, &instance_extensions, None)
        .expect("Couldn't create a new Vulkan instance")
}

fn create_surface(
    instance: Arc<Instance>,
    window: &Window,
) -> Arc<Surface<Sendable<Rc<WindowContext>>>> {
    let surface_handle = window
        .vulkan_create_surface(instance.internal_object().as_raw().try_into().unwrap())
        .expect("Couldn't create a new surface from the Vulkan Instance");
    // Use the SDL2 surface from the Window as surface
    unsafe {
        Arc::new(Surface::from_raw_surface(
            instance.clone(),
            ash::vk::SurfaceKHR::from_raw(surface_handle),
            Sendable::new(window.context()),
        ))
    }
}

fn get_device<'inst>(
    instance: &'inst Arc<Instance>,
    surface: Arc<Surface<Sendable<Rc<WindowContext>>>>,
) -> (PhysicalDevice<'inst>, Arc<Device>, Arc<Queue>) {
    let physical = PhysicalDevice::enumerate(&instance).next().unwrap();
    let queue_family = physical
        .queue_families()
        .find(|&q| q.supports_graphics() && surface.is_supported(q).unwrap_or(false))
        .expect("Couldn't generate queue family during Physical Device instancing");

    let device_ext = DeviceExtensions {
        khr_swapchain: true,
        ..DeviceExtensions::none()
    };
    let (device, mut queues) = Device::new(
        physical,
        physical.supported_features(),
        &device_ext,
        [(queue_family, 0.5)].iter().cloned(),
    )
    .expect("Couldn't create Vulkan Device");

    (
        physical,
        device,
        queues.next().expect("Couldn't get first queue object"),
    )
}

fn create_raw_swapchain(
    window: &Window,
    device: Arc<Device>,
    surface: Arc<Surface<Sendable<Rc<WindowContext>>>>,
    physical: PhysicalDevice,
) -> (
    Arc<Swapchain<Sendable<Rc<WindowContext>>>>,
    Vec<Arc<SwapchainImage<Sendable<Rc<WindowContext>>>>>,
) {
    // Get all the device capabilities and limitations
    let caps = surface
        .capabilities(physical)
        .expect("Couldn't obtain Vulkan Capabilities from Physical Device");
    let alpha = caps.supported_composite_alpha.iter().next().unwrap();
    let format = caps.supported_formats[0].0;

    let buffers_count = match caps.max_image_count {
        None => max(2, caps.min_image_count),
        Some(limit) => min(max(2, caps.min_image_count), limit),
    };
    let dimensions: [u32; 2] = {
        let size = window.size();
        [size.0, size.1]
    };
    Swapchain::start(device.clone(), surface.clone())
        .dimensions(dimensions)
        .usage(ImageUsage::color_attachment())
        .format(format)
        .composite_alpha(alpha)
        .num_images(buffers_count)
        .build()
        .expect("Couldn't build Vulkan Swapchain")
}

trait Draw {
    fn draw(
        &self,
        gl_handler: &mut GraphicsHandler,
        command_buffer: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    );
}

struct PrimitiveShape {
    vertex_buffer: VertexBuffer,
}

impl Draw for PrimitiveShape {
    fn draw(
        &self,
        gl_handler: &mut GraphicsHandler,
        command_buffer: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    ) {
        command_buffer
            .draw_indexed(
                gl_handler.get_pipeline("SimpleTriangle"),
                &gl_handler.get_swapchain().dynamic_state,
                self.vertex_buffer.get_vertices(),
                self.vertex_buffer.get_indices(),
                (),
                (),
                vec![],
            )
            .expect("Couldn't add Draw command to Vulkan Render Pass");
    }
}

type SpriteImmutableDescriptorSet = vulkano::descriptor::descriptor_set::PersistentDescriptorSet<(
    (
        (),
        PersistentDescriptorSetImg<Arc<ImageView<Arc<ImmutableImage>>>>,
    ),
    PersistentDescriptorSetSampler,
)>;

struct Sprite {
    vertex_buffer: VertexBuffer,
    immutable_descriptor_set: Arc<SpriteImmutableDescriptorSet>,
}

impl Sprite {
    fn new(texture_path: &str, gl_handler: &GraphicsHandler) -> Self {
        let vao = VertexArray::from(vec![
            Vertex {
                position: [-0.5, 0.5],
                color: [1.0, 0.0, 0.0],
            },
            Vertex {
                position: [-0.5, -0.5],
                color: [0.0, 0.0, 0.0],
            },
            Vertex {
                position: [0.5, -0.5],
                color: [0.0, 0.0, 1.0],
            },
            Vertex {
                position: [0.5, 0.5],
                color: [0.0, 0.0, 0.0],
            },
        ]);
        let indices = gl_handler.new_index_buffer(&[0, 1, 2, 2, 3, 0]);
        let vertex_buffer = gl_handler.new_vertex_buffer(vao, indices);


        let persistent_set = gl_handler.create_empty_descriptor_set_builder("Sprite", 0);
        let sampler = gl_handler.create_texture_sampler();

        let persistent_set = gl_handler
            .create_and_bind_texture(texture_path, persistent_set, sampler.clone())
            .build()
            .expect("Couldn't build Persistent Descriptor Set for Sprite object");

        let immutable_descriptor_set = Arc::new(persistent_set);

        Self {
            vertex_buffer,
            immutable_descriptor_set,
        }
    }
}

impl Draw for Sprite {
    fn draw(
        &self,
        gl_handler: &mut GraphicsHandler,
        command_buffer: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    ) {
        command_buffer
            .draw_indexed(
                gl_handler.get_pipeline("Sprite"),
                &gl_handler.get_swapchain().dynamic_state,
                self.vertex_buffer.get_vertices(),
                self.vertex_buffer.get_indices(),
                self.immutable_descriptor_set.clone(),
                (),
                vec![],
            )
            .expect("Couldn't add Draw command to Vulkan Render Pass");
    }
}

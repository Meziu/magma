// standard imports
use std::cell::RefCell;
use std::cmp::{max, min};
use std::collections::HashMap;
use std::convert::TryInto;
use std::fs::File;
use std::ops::DerefMut;
use std::rc::Rc;
use std::sync::Arc;

// Vulkano imports
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer, ImmutableBuffer, TypedBufferAccess};
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, DynamicState, SubpassContents,
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
use vulkano::instance::{Instance, InstanceExtensions, PhysicalDevice, PhysicalDeviceType};
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
use super::draw_objects::{Draw, DrawFlags, DrawObject, Sprite, SpriteObject, Primitive, PrimitiveObject};
use super::sendable::Sendable;
use cgmath::{Vector2, Vector4};
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
pub type DescriptorSetImg = PersistentDescriptorSetImg<Arc<ImageView<Arc<ImmutableImage>>>>;
pub type DescriptorSetWithImage<R> =
    PersistentDescriptorSetBuilder<((R, DescriptorSetImg), PersistentDescriptorSetSampler)>;
pub type GlobalUniformBuffer = CpuAccessibleBuffer<GlobalUniformData>;

/// Struct to hold the global data needed for graphics
#[derive(Clone, Copy)]
pub struct GlobalUniformData {
    window_size: Vector4<u32>,
    camera_position: Vector4<f32>,
    camera_scale: Vector4<f32>,
}

/// Struct to handle connections to the Vulkano (and thus Vulkan) API
pub struct GraphicsHandler {
    instance: Arc<Instance>,
    swapchain: SwapchainHandler,
    render_pass: Arc<RenderPass>,
    pipelines: HashMap<String, Arc<GraphicsPipeline<SingleBufferDefinition<Vertex>>>>,
    previous_frame_end: Option<Box<dyn GpuFuture>>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    draw_objects: Vec<DrawObject<dyn Draw>>,

    global_uniform_buffer: Arc<GlobalUniformBuffer>,
    pub window_size: Vector2<u32>,
    pub camera_position: Vector2<f32>,
    /// Zoom and stretch the whole view (If any of the dimensions is negative, it'll revert the view on that dimension)
    pub camera_scale: Vector2<f32>,
}

impl GraphicsHandler {
    /// Vulkan object handler instancing and init
    pub fn new(window: &Window) -> Self {
        let instance = create_instance();

        let surface = create_surface(instance.clone(), window);

        // Get the device info and queue
        let (physical, device, queue) = get_device(&instance, surface.clone());

        let (swapchain, images) = create_raw_swapchain(window, device.clone(), surface, physical);

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
            "Primitive",
            device,
            render_pass,
            "assets/shaders/primitive.vert",
            "assets/shaders/primitive.frag",
            &mut pipelines
        );
        create_pipeline!(
            "Sprite",
            device,
            render_pass,
            "assets/shaders/sprite.vert",
            "assets/shaders/sprite.frag",
            &mut pipelines
        );

        let swapchain = SwapchainHandler::new(swapchain, images, render_pass.clone());

        let previous_frame_end = Some(sync::now(device.clone()).boxed());

        let mut draw_objects = Vec::new();
        draw_objects.reserve(50);

        let window_size = window.size();
        let window_size = Vector2::new(window_size.0, window_size.1);
        let camera_position = Vector2::new(0.0, 0.0);
        let camera_scale = Vector2::new(1.0, 1.0);

        let global_uniform_data = GlobalUniformData {
            camera_position: camera_position.extend(0.0).extend(0.0),
            camera_scale: camera_scale.extend(0.0).extend(0.0),
            window_size: window_size.extend(0).extend(0),
        };
        let global_uniform_buffer = CpuAccessibleBuffer::from_data(
            device.clone(),
            BufferUsage::uniform_buffer_transfer_destination(),
            true,
            global_uniform_data,
        )
        .unwrap();

        Self {
            instance,
            swapchain,
            render_pass,
            pipelines,
            previous_frame_end,
            device,
            queue,
            draw_objects,

            global_uniform_buffer,
            window_size,
            camera_position,
            camera_scale,
        }
    }

    /// Rendering function to call every frame
    pub fn vulkan_loop(&mut self, resized: bool, window: &Window) {
        // Update the render object list and flush all the data to the gpu
        {
            self.draw_objects
                .retain(|o| o.borrow().read_flags().contains(DrawFlags::USED));
            self.flush_global_data();
            for o in &self.draw_objects {
                o.borrow().flush_data();
            }
        }

        // Check the window resize and make new framebuffers if needed
        {
            // If the window is being resized, return true, otherwise keep the original value (in case of pending resizes)
            let recreate: bool = {
                if resized {
                    self.window_size = window.size().into();
                    true
                } else {
                    self.swapchain.get_recreate()
                }
            };

            self.swapchain.set_recreate(recreate);

            let pass = self.render_pass.clone();
            let swapchain = self.get_swapchain();

            // Not an actual error, just a way to signify the need to retry the procedure
            if swapchain.check_and_recreate(window, pass).is_err() {
                return;
            }
        }

        // START OF THE ACTUAL LOOP

        // Get the future image
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

        // Create Command Buffer for draw calls
        let mut builder = AutoCommandBufferBuilder::primary(
            self.get_device(),
            self.queue.family(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .expect("Couldn't build Vulkan AutoCommandBuffer");

        // Initialize Command Buffer with the Render Pass
        builder
            .begin_render_pass(
                self.get_swapchain().framebuffers[image_num].clone(),
                SubpassContents::Inline,
                vec![[0.0, 0.0, 0.0, 1.0].into()],
            )
            .expect("Couldn't begin Vulkan Render Pass");

        // Filter all visible DrawObjects
        let cloned_list = self.draw_objects.clone();
        for obj in cloned_list
            .iter()
            .filter(|o| o.borrow().read_flags().contains(DrawFlags::VISIBLE))
        {
            // Draw object if visible
            obj.borrow_mut().draw(self, &mut builder);
        }

        // Build Command Buffer
        builder
            .end_render_pass()
            .expect("Couldn't properly end Vulkan Render Pass");
        let command_buffer = builder
            .build()
            .expect("Couldn't build Vulkan Command Buffer");

        // Run Command Buffer and obtain Future
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

        // Check the Future's output
        match future {
            Ok(future) => {
                // If the GPU is stuck rendering for too long terminate the program
                future
                    .wait(Some(std::time::Duration::from_secs(10)))
                    .expect("GPU Timeout, terminating the program");
                self.previous_frame_end = Some(future.boxed());
            }
            // Not a real error, may happen with weird Window resizing
            Err(FlushError::OutOfDate) => {
                self.get_swapchain().set_recreate(true);
                self.previous_frame_end = Some(sync::now(self.get_device()).boxed());
            }
            // Couldn't flush the future, big problem, pls fix yourself
            Err(e) => {
                eprintln!("Failed to flush Vulkan Future: {:?}", e);
                self.previous_frame_end = Some(sync::now(self.get_device()).boxed());
            }
        }

        // Clean the GpuFuture (unlock blocked memory and free remainings)
        self.previous_frame_end.as_mut().unwrap().cleanup_finished();
    }

    /// Sorter for the DrawObjects
    fn sort_draw_objects(&mut self) {
        self.draw_objects.sort_by(|a, b| {
            a.borrow_mut()
                .get_z_index()
                .cmp(&b.borrow_mut().get_z_index())
        });
    }

    /// Getter for the used Swapchain
    pub fn get_swapchain(&mut self) -> &mut SwapchainHandler {
        &mut self.swapchain
    }

    /// Getter for the used Device
    pub fn get_device(&self) -> Arc<Device> {
        self.device.clone()
    }

    /// Getter for a specific pipeline with a name
    pub fn get_pipeline(
        &self,
        name: &str,
    ) -> Arc<GraphicsPipeline<SingleBufferDefinition<Vertex>>> {
        self.pipelines
            .get(name)
            .expect("No Vulkan Pipeline under this name was found")
            .clone()
    }

    /// Getter for the Vulkan Queue
    fn get_queue(&self) -> Arc<Queue> {
        self.queue.clone()
    }

    /// Getter for the global uniform buffer
    pub fn get_global_uniform_buffer(&self) -> Arc<GlobalUniformBuffer> {
        self.global_uniform_buffer.clone()
    }

    /// Flusher for the global uniform buffer
    fn flush_global_data(&self) {
        let mut write_lock = self
            .global_uniform_buffer
            .write()
            .expect("Couldn't write global GPU buffer");
        let global_data = write_lock.deref_mut();

        global_data.window_size = self.window_size.extend(0).extend(0);
        global_data.camera_position = self.camera_position.extend(0.0).extend(0.0);
        global_data.camera_scale = self.camera_scale.extend(0.0).extend(0.0);
    }

    /// Create a new Immutable Vertex Buffer
    pub fn new_vertex_buffer(
        &self,
        vao: VertexArray,
        indices: Arc<dyn TypedBufferAccess<Content = [u16]> + Send + Sync>,
    ) -> VertexBuffer {
        VertexBuffer::new(self, vao, indices)
            .expect("Device Memory Allocation Error during creation of new Vertex Buffer")
    }

    /// Create a new Immutable Index Buffer (used to order the vertices on drawing)
    pub fn new_index_buffer(
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

    /// Create a new SpriteObject
    pub fn new_sprite(&mut self, texture_path: &str, z_index: u8) -> SpriteObject {
        let sprite = Rc::new(RefCell::new(Sprite::new(texture_path, self, z_index)));

        self.append_draw_object(sprite.clone());

        SpriteObject::new(sprite)
    }

    /// Create a new rectangular PrimitiveObject
    pub fn new_rectangle(&mut self, scale: Vector2<f32>, color: Vector4<f32>, global_position: Vector2<f32>, z_index: u8) -> PrimitiveObject {
        let primitive = Rc::new(RefCell::new(Primitive::rectangle(scale, color, global_position, self, z_index)));

        self.append_draw_object(primitive.clone());

        PrimitiveObject::new(primitive)
    }

    /// Append a new DrawObject to the draw_object vector for draw
    fn append_draw_object(&mut self, obj: DrawObject<dyn Draw>) {
        self.draw_objects.push(obj);
        self.sort_draw_objects();
    }

    /// Create a new empty Immutable Descriptor Set
    pub fn create_empty_descriptor_set_builder(
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

    /// Bind a texture to a new Immutable Descriptor Set
    pub fn create_and_bind_texture<R>(
        &self,
        texture_path: &str,
        desc_set_builder: PersistentDescriptorSetBuilder<R>,
        sampler: Arc<Sampler>,
    ) -> (
        DescriptorSetWithImage<R>,
        Vector2<u32>,
    ) {
        let decoder = png::Decoder::new(File::open(texture_path).unwrap());
        let (info, mut reader) = decoder.read_info().unwrap();

        let mut buf = vec![0; info.buffer_size()];

        reader.next_frame(&mut buf).unwrap();

        let dimensions = ImageDimensions::Dim2d {
            width: info.width,
            height: info.height,
            array_layers: 1,
        };
        let (image, future) = ImmutableImage::from_iter(
            buf.iter().cloned(),
            dimensions,
            MipmapsCount::One,
            Format::R8G8B8A8Srgb,
            self.get_queue(),
        )
        .unwrap();

        let (texture, _tex_future) = (ImageView::new(image).unwrap(), future);

        (
            desc_set_builder
                .add_sampled_image(texture, sampler)
                .expect("Couldn't add Sampled Image to Descriptor Set"),
            Vector2::new(info.width, info.height),
        )
    }

    /// Create a Texture Sampler to bind Textures to
    pub fn create_texture_sampler(&self) -> Arc<Sampler> {
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
pub struct SwapchainHandler {
    chain: Arc<Swapchain<Sendable<Rc<WindowContext>>>>,
    images: Vec<Arc<SwapchainImage<Sendable<Rc<WindowContext>>>>>,
    framebuffers: Vec<Arc<dyn FramebufferAbstract + Send + Sync>>,
    must_recreate: bool,
    dynamic_state: Box<DynamicState>,
}

impl SwapchainHandler {
    fn new(
        swapchain: Arc<Swapchain<Sendable<Rc<WindowContext>>>>,
        images: Vec<Arc<SwapchainImage<Sendable<Rc<WindowContext>>>>>,
        render_pass: Arc<RenderPass>,
    ) -> Self {
        let mut dynamic_state = Box::new(DynamicState {
            line_width: None,
            viewports: None,
            scissors: None,
            compare_mask: None,
            write_mask: None,
            reference: None,
        });

        let framebuffers =
            window_size_dependent_setup(&images[..], render_pass, dynamic_state.as_mut());

        Self {
            chain: swapchain,
            images,
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

            let framebuffers =
                window_size_dependent_setup(&self.images[..], pass, &mut self.dynamic_state);
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

    pub fn get_dynamic_state(&mut self) -> &mut DynamicState {
        self.dynamic_state.as_mut()
    }
}

/// Struct to hold vertex data
#[derive(Default, Copy, Clone)]
pub struct Vertex {
    pub vert_pos: [f32; 2],
}
vulkano::impl_vertex!(Vertex, vert_pos);

/// Simple struct to hold an array of vertices
pub struct VertexArray {
    data: Vec<Vertex>,
}

impl From<Vec<Vertex>> for VertexArray {
    fn from(vec: Vec<Vertex>) -> Self {
        Self { data: vec }
    }
}

/// Struct to hold a vertex buffer with data
#[derive(Clone)]
pub struct VertexBuffer {
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
            instance,
            ash::vk::SurfaceKHR::from_raw(surface_handle),
            Sendable::new(window.context()),
        ))
    }
}

fn get_device(
    instance: &'_ Arc<Instance>,
    surface: Arc<Surface<Sendable<Rc<WindowContext>>>>,
) -> (PhysicalDevice<'_>, Arc<Device>, Arc<Queue>) {
    let (physical_device, queue_family) = PhysicalDevice::enumerate(&instance)
        .filter_map(|p| {
            p.queue_families()
                .find(|&q| q.supports_graphics() && surface.is_supported(q).unwrap_or(false))
                .map(|q| (p, q))
        })
        .min_by_key(|(p, _)| match p.properties().device_type.unwrap() {
            PhysicalDeviceType::DiscreteGpu => 0,
            PhysicalDeviceType::IntegratedGpu => 1,
            PhysicalDeviceType::VirtualGpu => 2,
            PhysicalDeviceType::Cpu => 3,
            PhysicalDeviceType::Other => 4,
        })
        .unwrap();

    let device_ext = DeviceExtensions {
        khr_swapchain: true,
        ..DeviceExtensions::none()
    };
    let (device, mut queues) = Device::new(
        physical_device,
        physical_device.supported_features(),
        &device_ext,
        [(queue_family, 0.5)].iter().cloned(),
    )
    .expect("Couldn't create Vulkan Device");

    (
        physical_device,
        device,
        queues.next().expect("Couldn't get first queue object"),
    )
}

type SdlSwapchain = Arc<Swapchain<Sendable<Rc<WindowContext>>>>;
type SdlSwapchainImagesVector = Vec<Arc<SwapchainImage<Sendable<Rc<WindowContext>>>>>;

fn create_raw_swapchain(
    window: &Window,
    device: Arc<Device>,
    surface: Arc<Surface<Sendable<Rc<WindowContext>>>>,
    physical: PhysicalDevice,
) -> (
    SdlSwapchain,
    SdlSwapchainImagesVector,
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
    Swapchain::start(device, surface)
        .dimensions(dimensions)
        .usage(ImageUsage::color_attachment())
        .format(format)
        .composite_alpha(alpha)
        .num_images(buffers_count)
        .build()
        .expect("Couldn't build Vulkan Swapchain")
}

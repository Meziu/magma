// standard imports
use std::cmp::{max, min};
use std::collections::HashMap;
use std::ffi::CString;
use std::rc::Rc;
use std::sync::Arc;

// Vulkano imports
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, DynamicState, SubpassContents,
};
use vulkano::descriptor::PipelineLayoutAbstract;
use vulkano::device::{Device, DeviceExtensions, Queue};
use vulkano::image::view::ImageView;
use vulkano::image::{ImageUsage, SwapchainImage};
use vulkano::instance::{Instance, PhysicalDevice, RawInstanceExtensions};
use vulkano::memory::DeviceMemoryAllocError;
use vulkano::pipeline::vertex::SingleBufferDefinition;
use vulkano::pipeline::viewport::Viewport;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::render_pass::RenderPass;
use vulkano::render_pass::{Framebuffer, FramebufferAbstract, Subpass};
use vulkano::swapchain;
use vulkano::swapchain::{AcquireError, Surface, Swapchain, SwapchainCreationError};
use vulkano::sync;
use vulkano::sync::{FlushError, GpuFuture};
use vulkano::VulkanObject;

// SDL2 imports
use sdl2::video::{Window, WindowContext};

// other imports
use super::sendable::Sendable;

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
                .triangle_list()
                .viewports_dynamic_scissors_irrelevant(1)
                .fragment_shader(frag_shader.main_entry_point(), ())
                .render_pass(Subpass::from($render_pass.clone(), 0).unwrap())
                .build($device.clone())
                .expect("Couldn't create new Vulkan Graphics Pipeline"),
        );
        $map.insert($name.to_string(), pipeline.clone());
    };};
}

/// Struct to handle connections to the Vulkano (and thus Vulkan) API
pub struct GraphicsHandler {
    instance: Arc<Instance>,
    swapchain: SwapchainHandler,
    render_pass: Arc<RenderPass>,
    pipelines: HashMap<
        String,
        Arc<
            GraphicsPipeline<
                SingleBufferDefinition<Vertex>,
                Box<dyn PipelineLayoutAbstract + Send + Sync>,
            >,
        >,
    >,
    previous_frame_end: Option<Box<dyn GpuFuture>>,
    device: Arc<Device>,
    queue: Arc<Queue>,
}

impl GraphicsHandler {
    /// Vulkan object handler instancing and init
    pub fn new(window: &Window) -> Self {
        let instance = create_instance(window);

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

        let clear_values = vec![[0.0, 0.0, 1.0, 1.0].into()];

        let mut builder = AutoCommandBufferBuilder::primary(
            self.device.clone(),
            self.queue.family(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .expect("Couldn't build Vulkan AutoCommandBuffer");

        let vao = VertexArray::from(vec![
            Vertex {
                position: [-0.5, -0.25],
            },
            Vertex {
                position: [0.0, 0.5],
            },
            Vertex {
                position: [0.25, -0.1],
            },
        ]);
        let vb = self.new_vertex_buffer(vao);

        builder
            .begin_render_pass(
                self.get_swapchain().framebuffers[image_num].clone(),
                SubpassContents::Inline,
                clear_values,
            )
            .expect("Couldn't begin Vulkan Render Pass")
            .draw(
                self.pipelines
                    .get(&"SimpleTriangle".to_string())
                    .expect("No Vulkan Pipeline under this name was found")
                    .clone(),
                &self.get_swapchain().dynamic_state,
                vb.buffer.clone(),
                (),
                (),
                vec![],
            )
            .expect("Couldn't add Draw command to Vulkan Render Pass")
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
                self.previous_frame_end = Some(sync::now(self.device.clone()).boxed());
            }
            Err(e) => {
                eprintln!("Failed to flush Vulkan Future: {:?}", e);
                self.previous_frame_end = Some(sync::now(self.device.clone()).boxed());
            }
        }
    }

    fn get_swapchain(&mut self) -> &mut SwapchainHandler {
        &mut self.swapchain
    }

    fn new_vertex_buffer(&self, vao: VertexArray) -> VertexBuffer {
        VertexBuffer::new(self.device.clone(), vao)
            .expect("Device Memory Allocation Error during creation of new Vertex Buffer")
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
}
vulkano::impl_vertex!(Vertex, position);

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
    buffer: Arc<CpuAccessibleBuffer<[Vertex]>>,
}

impl VertexBuffer {
    pub fn new(device: Arc<Device>, array: VertexArray) -> Result<Self, DeviceMemoryAllocError> {
        let buffer = CpuAccessibleBuffer::from_iter(
            device,
            BufferUsage::all(),
            false,
            array.data.iter().cloned(),
        )?;

        Ok(Self { buffer })
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

fn create_instance(window: &Window) -> Arc<Instance> {
    let instance_extensions = window
        .vulkan_instance_extensions()
        .expect("Couldn't obtain Vulkan Instance Extensions from the Window");
    let raw_instance_extensions = RawInstanceExtensions::new(
        instance_extensions
            .iter()
            .map(|&v| CString::new(v).unwrap()),
    );

    Instance::new(None, raw_instance_extensions, None)
        .expect("Couldn't create a new Vulkan instance")
}

fn create_surface(
    instance: Arc<Instance>,
    window: &Window,
) -> Arc<Surface<Sendable<Rc<WindowContext>>>> {
    let surface_handle = window
        .vulkan_create_surface(instance.internal_object())
        .expect("Couldn't create a new surface from the Vulkan Instance");
    // Use the SDL2 surface from the Window as surface
    unsafe {
        Arc::new(Surface::from_raw_surface(
            instance.clone(),
            surface_handle,
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

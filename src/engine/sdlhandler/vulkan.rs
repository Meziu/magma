// standard imports
use std::cmp::{max, min};
use std::error::Error;
use std::ffi::CString;
use std::rc::Rc;
use std::sync::Arc;

// Vulkano imports
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{AutoCommandBufferBuilder, DynamicState, SubpassContents, CommandBufferUsage};
use vulkano::device::{Device, DeviceExtensions, Queue};
use vulkano::image::{ImageUsage, SwapchainImage};
use vulkano::image::view::ImageView;
use vulkano::instance::{Instance, PhysicalDevice, RawInstanceExtensions};
use vulkano::pipeline::viewport::Viewport;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::pipeline::vertex::SingleBufferDefinition;
use vulkano::render_pass::{Framebuffer, FramebufferAbstract, Subpass};
use vulkano::swapchain;
use vulkano::swapchain::{AcquireError, Surface, Swapchain, SwapchainCreationError};
use vulkano::sync;
use vulkano::sync::{FlushError, GpuFuture,};
use vulkano::VulkanObject;
use vulkano::render_pass::RenderPass;
use vulkano::descriptor::PipelineLayoutAbstract;

// SDL2 imports
use sdl2::video::{Window, WindowContext};

// other imports
mod sendable;
use sendable::Sendable;


/// Struct to handle connections to the Vulkano (and thus Vulkan) API
pub struct GraphicsHandler {
    instance: Arc<Instance>,
    swapchain: SwapchainHandler,
    render_pass: Arc<RenderPass>,
    pipeline: Arc<GraphicsPipeline<SingleBufferDefinition<Vertex>, Box<dyn PipelineLayoutAbstract + Send + Sync>>>,
    previous_frame_end: Option<Box<dyn GpuFuture>>,
    device: Arc<Device>,
    queue: Arc<Queue>,
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
    pub fn check_and_recreate(&mut self, window: &Window, pass: Arc<RenderPass>) -> Result<(), ()>{
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
                window_size_dependent_setup(&self.images[..], pass.clone(), &mut self.dynamic_state);
            
            self.framebuffers = framebuffers;
            self.must_recreate = false;
        }
        Ok(())
    }

    pub fn get_recreate(&self) -> bool {
        self.must_recreate
    }

    pub fn set_recreate(&mut self, new_value: bool) {
        self.must_recreate = new_value;
    }
}

impl GraphicsHandler {
    pub fn new(window: &Window) -> Result<GraphicsHandler, Box<dyn Error>> {
        // Vulkan instancing and init
        let instance_extensions = window.vulkan_instance_extensions()?;
        let raw_instance_extensions = RawInstanceExtensions::new(
            instance_extensions
                .iter()
                .map(|&v| CString::new(v).unwrap()),
        );

        let instance = Instance::new(None, raw_instance_extensions, None)?;

        let surface_handle = window.vulkan_create_surface(instance.internal_object())?;
        // Use the SDL2 surface from the Window as surface
        let surface = unsafe {
            Arc::new(Surface::from_raw_surface(
                instance.clone(),
                surface_handle,
                Sendable::new(window.context()),
            ))
        };

        // Get the device info and queue
        let physical = PhysicalDevice::enumerate(&instance).next().unwrap();
        let queue_family = physical
            .queue_families()
            .find(|&q| q.supports_graphics() && surface.is_supported(q).unwrap_or(false))
            .unwrap();

        let device_ext = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::none()
        };
        let (device, mut queues) = Device::new(
            physical,
            physical.supported_features(),
            &device_ext,
            [(queue_family, 0.5)].iter().cloned(),
        )?;

        let queue = queues.next().unwrap();

        let (swapchain, images) = {
            // Get all the device capabilities and limitations
            let caps = surface.capabilities(physical)?;
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
                .build()?
        };


        let vs = vs::Shader::load(device.clone())?;
        let fs = fs::Shader::load(device.clone())?;

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
            )?,
        );

        let pipeline = Arc::new(
            GraphicsPipeline::start()
                .vertex_input_single_buffer()
                .vertex_shader(vs.main_entry_point(), ())
                .triangle_list()
                .viewports_dynamic_scissors_irrelevant(1)
                .fragment_shader(fs.main_entry_point(), ())
                .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
                .build(device.clone())?
        );

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

        let swapchain = SwapchainHandler {
            chain: swapchain,
            images: images,
            framebuffers,
            must_recreate: false,
            dynamic_state,
        };

        let previous_frame_end = Some(sync::now(device.clone()).boxed());
        
        Ok(GraphicsHandler {
            instance: instance.clone(),
            swapchain,
            render_pass: render_pass.clone(),
            pipeline: pipeline.clone(),
            previous_frame_end,
            device,
            queue,
        })
    }

    pub fn vulkan_loop(&mut self, resized: bool, window: &Window) -> Result<(), Box<dyn Error>> {
        {
            // If the window is being resized, return true, otherwise keep the original value (in case of pending resizes)
            let recreate: bool = {
                if resized {
                    true
                } else { self.swapchain.get_recreate() }
            };

            self.swapchain.set_recreate(recreate);

            let pass = self.render_pass.clone();
            let swapchain = self.get_swapchain();

            // Not an actual error, just a way to signify the need to retry the procedure
            if let Err(_) = swapchain.check_and_recreate(window, pass) {
                return Ok(())
            }
        }
        // start of the actual loop code
        self.previous_frame_end.as_mut().unwrap().cleanup_finished();
        let (image_num, suboptimal, acquire_future) =
            match swapchain::acquire_next_image(self.get_swapchain().chain.clone(), None) {
                Ok(r) => r,
                Err(AcquireError::OutOfDate) => {
                    self.get_swapchain().set_recreate(true);
                    return Ok(());
                }
                Err(e) => return Err(Box::new(e)),
            };
        self.get_swapchain().set_recreate(suboptimal);


        let clear_values = vec![[0.0, 0.0, 1.0, 1.0].into()];

        let mut builder = AutoCommandBufferBuilder::primary(
            self.device.clone(),
            self.queue.family(),
            CommandBufferUsage::OneTimeSubmit,
        )?;

        let vao = VertexArray::new(vec!(
            Vertex {
                position: [-0.5, -0.25],
            },
            Vertex {
                position: [0.0, 0.5],
            },
            Vertex {
                position: [0.25, -0.1],
            },
        ))?;
        let vb = VertexBuffer::new(self.device.clone(), vao)?;

        builder
        .begin_render_pass(
            self.get_swapchain().framebuffers[image_num].clone(),
            SubpassContents::Inline,
            clear_values,
        )?
        .draw(
            self.pipeline.clone(),
            &self.get_swapchain().dynamic_state,
            vb.buffer.clone(),
            (),
            (),
            vec![],
        )?
        .end_render_pass()?;

        let command_buffer = builder.build()?;

        let future = self.previous_frame_end
            .take()
            .unwrap()
            .join(acquire_future)
            .then_execute(self.queue.clone(), command_buffer)?
            .then_swapchain_present(self.queue.clone(), self.get_swapchain().chain.clone(), image_num)
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
                println!("Failed to flush future: {:?}", e);
                self.previous_frame_end = Some(sync::now(self.device.clone()).boxed());
            }
        }
        Ok(())
    }

    fn get_swapchain(&mut self) -> &mut SwapchainHandler {
        &mut self.swapchain
    }
}


/// Struct to hold vertex data
#[derive(Default, Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}
vulkano::impl_vertex!(Vertex, position);

/// Simple struct to hold an array of objects
struct VertexArray {
    data: Vec<Vertex>,
}

impl VertexArray {
    pub fn new(data: Vec<Vertex>) -> Result<VertexArray, Box<dyn Error>> {
        Ok(
            VertexArray{
                data,
            }
        )
    }
}

/// Struct to hold a vertex buffer with data
struct VertexBuffer {
    buffer: Arc<CpuAccessibleBuffer<[Vertex]>>,
}

impl VertexBuffer {
    pub fn new(device: Arc<Device>, array: VertexArray) -> Result<VertexBuffer, Box<dyn Error>> {
        let buffer = CpuAccessibleBuffer::from_iter(
            device,
            BufferUsage::all(),
            false,
            array.data.iter().cloned(),
        )?;

        Ok(
            VertexBuffer{
                buffer,
            }
        )
    }
}


mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: "
            #version 450

            layout(location = 0) in vec2 position;

            void main() {
                gl_Position = vec4(position, 0.0, 1.0);
            }
        "
    }
}

mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: "
            #version 450

            layout(location = 0) out vec4 f_color;

            void main() {
                f_color = vec4(1.0, 0.0, 0.0, 1.0);
            }
        "
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
            let view = ImageView::new(image.clone()).unwrap();
            Arc::new(
                Framebuffer::start(render_pass.clone())
                    .add(view)
                    .unwrap()
                    .build()
                    .unwrap(),
            ) as Arc<dyn FramebufferAbstract + Send + Sync>
        })
        .collect::<Vec<_>>()
}
// standard imports
use std::sync::Arc;

// vulkan imports
use vulkano::buffer::{ImmutableBuffer, TypedBufferAccess};
use vulkano::command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer};
use vulkano::descriptor::descriptor_set::collection::DescriptorSetsCollection;
use vulkano::descriptor::descriptor_set::{
    PersistentDescriptorSetImg, PersistentDescriptorSetSampler,
};
use vulkano::image::view::ImageView;
use vulkano::image::ImmutableImage;
use vulkano::pipeline::vertex::SingleBufferDefinition;
use vulkano::pipeline::GraphicsPipeline;

// vulkan implementation imports
use super::vulkan::{GraphicsHandler, Vertex, VertexArray, VertexBuffer};

pub trait Draw {
    fn draw(
        &self,
        gl_handler: &mut GraphicsHandler,
        command_buffer: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    );

    fn get_z_index(&self) -> u8;
}

pub struct PrimitiveShape {
    vertex_buffer: VertexBuffer,
}

impl Draw for PrimitiveShape {
    fn draw(
        &self,
        gl_handler: &mut GraphicsHandler,
        command_buffer: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    ) {
        draw(
            gl_handler,
            gl_handler.get_pipeline("Primitive"),
            command_buffer,
            self.vertex_buffer.get_vertices(),
            self.vertex_buffer.get_indices(),
            (),
        )
    }

    fn get_z_index(&self) -> u8 {
        0
    }
}

type SpriteImmutableDescriptorSet = vulkano::descriptor::descriptor_set::PersistentDescriptorSet<(
    (
        (),
        PersistentDescriptorSetImg<Arc<ImageView<Arc<ImmutableImage>>>>,
    ),
    PersistentDescriptorSetSampler,
)>;

/// Struct to handle sprite entities on screen capable of having transforms
pub struct Sprite {
    vertex_buffer: VertexBuffer,
    immutable_descriptor_set: Arc<SpriteImmutableDescriptorSet>,
    z_index: u8,
}

impl Sprite {
    pub fn new(texture_path: &str, gl_handler: &GraphicsHandler, z_index: u8) -> Self {
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
            z_index,
        }
    }
}

impl Draw for Sprite {
    fn draw(
        &self,
        gl_handler: &mut GraphicsHandler,
        command_buffer: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    ) {
        draw(
            gl_handler,
            gl_handler.get_pipeline("Sprite"),
            command_buffer,
            self.vertex_buffer.get_vertices(),
            self.vertex_buffer.get_indices(),
            vec![self.immutable_descriptor_set.clone()],
        )
    }

    fn get_z_index(&self) -> u8 {
        self.z_index
    }
}

fn draw<DescSet>(
    gl_handler: &mut GraphicsHandler,
    pipeline: Arc<GraphicsPipeline<SingleBufferDefinition<Vertex>>>,
    cmnd_buf: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    vertices: Arc<ImmutableBuffer<[Vertex]>>,
    indices: Arc<dyn TypedBufferAccess<Content = [u16]> + Send + Sync>,
    sets: DescSet,
) where
    DescSet: DescriptorSetsCollection,
{
    cmnd_buf
        .draw_indexed(
            pipeline,
            &gl_handler.get_swapchain().get_dynamic_state(),
            vertices,
            indices,
            sets,
            (),
            vec![],
        )
        .expect("Couldn't add Draw command to Vulkan Render Pass");
}

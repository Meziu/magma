// standard imports
use std::sync::Arc;

// vulkan imports
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, PrimaryAutoCommandBuffer,
};
use vulkano::descriptor::descriptor_set::{PersistentDescriptorSetImg,PersistentDescriptorSetSampler,
};
use vulkano::image::view::ImageView;
use vulkano::image::ImmutableImage;

// vulkan implementation imports
use super::vulkan::{GraphicsHandler, VertexBuffer, Vertex, VertexArray};


pub trait Draw {
    fn draw(
        &self,
        gl_handler: &mut GraphicsHandler,
        command_buffer: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    );
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
        command_buffer
            .draw_indexed(
                gl_handler.get_pipeline("Primitive"),
                &gl_handler.get_swapchain().get_dynamic_state(),
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

pub struct Sprite {
    vertex_buffer: VertexBuffer,
    immutable_descriptor_set: Arc<SpriteImmutableDescriptorSet>,
}

impl Sprite {
    pub fn new(texture_path: &str, gl_handler: &GraphicsHandler) -> Self {
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
                &gl_handler.get_swapchain().get_dynamic_state(),
                self.vertex_buffer.get_vertices(),
                self.vertex_buffer.get_indices(),
                self.immutable_descriptor_set.clone(),
                (),
                vec![],
            )
            .expect("Couldn't add Draw command to Vulkan Render Pass");
    }
}
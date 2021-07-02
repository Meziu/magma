// standard imports
use std::ops::DerefMut;
use std::sync::Arc;

// vulkan imports
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer, ImmutableBuffer, TypedBufferAccess};
use vulkano::command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer};
use vulkano::descriptor::descriptor_set::collection::DescriptorSetsCollection;
use vulkano::descriptor::descriptor_set::{
    PersistentDescriptorSetBuf, PersistentDescriptorSetImg, PersistentDescriptorSetSampler, PersistentDescriptorSet,
};
use vulkano::image::view::ImageView;
use vulkano::image::ImmutableImage;
use vulkano::pipeline::vertex::SingleBufferDefinition;
use vulkano::pipeline::GraphicsPipeline;

// vulkan implementation imports
use super::vulkan::{GlobalUniformData, GraphicsHandler, Vertex, VertexArray, VertexBuffer};

// other imports
use cgmath::{Vector2, Vector4};

pub trait Draw {
    fn draw(
        &self,
        gl_handler: &mut GraphicsHandler,
        command_buffer: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    );

    fn get_z_index(&self) -> u8;

    fn flush_data(&self);
}

/// Struct for User generated shapes
/// DO NOT USE, IT'S NOT UPDATED
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

    fn flush_data(&self) {

    }
}

type SpriteImmutableDescriptorSet = PersistentDescriptorSet<(
    (
        (
            (
                (),
                PersistentDescriptorSetImg<
                    Arc<
                        ImageView<
                            Arc<ImmutableImage>,
                        >,
                    >,
                >,
            ),
            PersistentDescriptorSetSampler,
        ),
        PersistentDescriptorSetBuf<
            Arc<
                CpuAccessibleBuffer<SpriteData>,
            >,
        >,
    ),
    PersistentDescriptorSetBuf<
        Arc<
            CpuAccessibleBuffer<GlobalUniformData>,
        >,
    >,
)>;

/// Struct to hold data that both CPU and GPU must access
#[derive(Copy, Clone, Debug)]
struct SpriteData {
    color: Vector4<f32>,
    global_position: Vector4<f32>,
    scale: Vector4<f32>,
    image_dimensions: Vector4<u32>,
}

/// Struct to handle sprite entities on screen capable of having transforms
#[derive(Clone)]
pub struct Sprite {
    vertex_buffer: VertexBuffer,
    descriptor_set: Arc<SpriteImmutableDescriptorSet>,
    cpu_buffer: Arc<CpuAccessibleBuffer<SpriteData>>,
    z_index: u8,

    pub color: Vector4<f32>,
    pub global_position: Vector2<f32>,
    pub scale: Vector2<f32>,
    pub image_dimensions: Vector2<u32>,
}

impl Sprite {
    pub fn new(texture_path: &str, gl_handler: &GraphicsHandler, z_index: u8) -> Self {
        let vao = VertexArray::from(vec![
            Vertex {
                vert_pos: [-1.0, -1.0],
            },
            Vertex {
                vert_pos: [-1.0, 1.0],
            },
            Vertex {
                vert_pos: [1.0, 1.0],
            },
            Vertex {
                vert_pos: [1.0, -1.0],
            },
        ]);
        let indices = gl_handler.new_index_buffer(&[0, 1, 2, 2, 3, 0]);
        let vertex_buffer = gl_handler.new_vertex_buffer(vao, indices);

        let persistent_set = gl_handler.create_empty_descriptor_set_builder("Sprite", 0);
        let sampler = gl_handler.create_texture_sampler();

        let color = Vector4::new(1.0, 1.0, 1.0, 1.0);
        let global_position = Vector2::new(0.0, 0.0);
        let scale = Vector2::new(1.0, 1.0);

        let (persistent_set, image_dimensions) =
            gl_handler.create_and_bind_texture(texture_path, persistent_set, sampler.clone());

        let sprite_data = SpriteData {
            global_position: global_position.extend(0.0).extend(0.0),
            color,
            scale: scale.extend(0.0).extend(0.0),
            image_dimensions: image_dimensions.extend(0).extend(0),
        };

        let cpu_buffer = CpuAccessibleBuffer::from_data(
            gl_handler.get_device(),
            BufferUsage::uniform_buffer(),
            true,
            sprite_data,
        )
        .unwrap();

        let persistent_set = persistent_set
            .add_buffer(cpu_buffer.clone())
            .unwrap()
            .add_buffer(gl_handler.get_global_uniform_buffer())
            .unwrap()
            .build()
            .expect("Couldn't build Persistent Descriptor Set for Sprite object");

        let descriptor_set = Arc::new(persistent_set);

        Self {
            vertex_buffer,
            descriptor_set,
            cpu_buffer,
            z_index,
            color,
            image_dimensions,
            scale,
            global_position,
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
            self.descriptor_set.clone(),
        )
    }

    fn get_z_index(&self) -> u8 {
        self.z_index
    }

    fn flush_data(&self) {
        let mut write_lock = self.cpu_buffer.write().expect("Couldn't write the buffer");
        let sprite_data = write_lock.deref_mut();

        sprite_data.color = self.color;
        sprite_data.global_position = self.global_position.extend(0.0).extend(0.0);
        sprite_data.scale = self.scale.extend(0.0).extend(0.0);
        // sprite_data.image_dimensions = self.image_dimensions.extend(0).extend(0);
        // image dimensions can't change, maybe with Animated Sprites it could
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

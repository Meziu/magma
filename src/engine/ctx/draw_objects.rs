// standard imports
use std::ops::{Deref, DerefMut};
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
pub struct Sprite {
    vertex_buffer: VertexBuffer,
    descriptor_set: Arc<SpriteImmutableDescriptorSet>,
    cpu_buffer: Arc<CpuAccessibleBuffer<SpriteData>>,
    z_index: u8,
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
        let global_position = Vector4::new(0.0, 0.0, 0.0, 0.0);
        let scale = Vector4::new(1.0, 1.0, 0.0, 0.0);

        let (persistent_set, image_dimensions) =
            gl_handler.create_and_bind_texture(texture_path, persistent_set, sampler.clone());

        let image_dimensions = image_dimensions.extend(0).extend(0);
        let sprite_data = SpriteData {
            global_position,
            color,
            scale,
            image_dimensions,
        };

        let cpu_buffer = CpuAccessibleBuffer::from_data(
            gl_handler.get_device(),
            BufferUsage::all(),
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
        }
    }

    pub fn set_color(&self, new_color: Vector4<f32>) {
        let mut write_lock = self.cpu_buffer.write().expect("Couldn't write the buffer");
        let sprite_data = write_lock.deref_mut();

        sprite_data.color = new_color;
    }

    pub fn get_color(&self) -> Vector4<f32> {
        let read_lock = self.cpu_buffer.read().expect("Couldn't read the buffer");
        let sprite_data = read_lock.deref();

        sprite_data.color.clone()
    }

    pub fn set_global_position(&self, new_position: Vector2<f32>) {
        let mut write_lock = self.cpu_buffer.write().expect("Couldn't write the buffer");
        let sprite_data = write_lock.deref_mut();

        sprite_data.global_position = new_position.extend(0.0).extend(0.0);
    }

    pub fn get_global_position(&self) -> Vector2<f32> {
        let read_lock = self.cpu_buffer.read().expect("Couldn't read the buffer");
        let sprite_data = read_lock.deref();

        sprite_data.global_position.clone().truncate().truncate()
    }

    pub fn set_scale(&self, new_scale: Vector2<f32>) {
        let mut write_lock = self.cpu_buffer.write().expect("Couldn't write the buffer");
        let sprite_data = write_lock.deref_mut();

        sprite_data.scale = new_scale.extend(0.0).extend(0.0);
    }

    pub fn get_scale(&self) -> Vector2<f32> {
        let read_lock = self.cpu_buffer.read().expect("Couldn't read the buffer");
        let sprite_data = read_lock.deref();

        sprite_data.scale.clone().truncate().truncate()
    }

    pub fn get_image_dimensions(&self) -> Vector2<u32> {
        let read_lock = self.cpu_buffer.read().expect("Couldn't read the buffer");
        let sprite_data = read_lock.deref();

        sprite_data.image_dimensions.clone().truncate().truncate()
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

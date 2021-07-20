// standard imports
use std::cell::RefCell;
use std::cell::{Ref, RefMut};
use std::ops::DerefMut;
use std::rc::Rc;
use std::sync::Arc;

// vulkan imports
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer, ImmutableBuffer, TypedBufferAccess};
use vulkano::command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer};
use vulkano::descriptor::descriptor_set::collection::DescriptorSetsCollection;
use vulkano::descriptor::descriptor_set::{
    PersistentDescriptorSet, PersistentDescriptorSetBuf, PersistentDescriptorSetImg,
    PersistentDescriptorSetSampler,
};
use vulkano::image::view::ImageView;
use vulkano::image::ImmutableImage;
use vulkano::pipeline::vertex::SingleBufferDefinition;
use vulkano::pipeline::GraphicsPipeline;

// vulkan implementation imports
use super::vulkan::{GlobalUniformData, GraphicsHandler, Vertex, VertexArray, VertexBuffer};

// other imports
use bitflags::bitflags;
use cgmath::{Vector2, Vector4};

bitflags! {
    pub struct DrawFlags: u8 {
        const USED = 0b00000001;
        const VISIBLE = 0b00000010;
    }
}

pub trait Draw {
    fn draw(
        &self,
        gl_handler: &mut GraphicsHandler,
        command_buffer: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    );

    fn get_z_index(&self) -> u8;

    fn flush_data(&self);

    fn write_flags(&mut self) -> &mut DrawFlags;
    fn read_flags(&self) -> DrawFlags;

    fn set_dead(&mut self);
    fn set_visible(&mut self, visible: bool);
}

pub type DrawObject<O> = Rc<RefCell<O>>;

pub type SpriteObject = GraphicObject<Sprite>;

type SpriteImmutableDescriptorSet = PersistentDescriptorSet<(
    (
        (
            (
                (),
                PersistentDescriptorSetImg<Arc<ImageView<Arc<ImmutableImage>>>>,
            ),
            PersistentDescriptorSetSampler,
        ),
        PersistentDescriptorSetBuf<Arc<CpuAccessibleBuffer<SpriteData>>>,
    ),
    PersistentDescriptorSetBuf<Arc<CpuAccessibleBuffer<GlobalUniformData>>>,
)>;

/// User Accessible DrawObject dependent on the draw type
pub struct GraphicObject<O: Draw + ?Sized> {
    draw_object: DrawObject<O>,
}

impl<O: Draw + ?Sized> GraphicObject<O> {
    pub fn new(draw_object: DrawObject<O>) -> Self {
        Self { draw_object }
    }

    pub fn get_ref(&self) -> Ref<'_, O> {
        self.draw_object.borrow()
    }

    pub fn get_mut(&self) -> RefMut<'_, O> {
        self.draw_object.borrow_mut()
    }
}

impl<O: Draw + ?Sized> Drop for GraphicObject<O> {
    fn drop(&mut self) {
        self.draw_object.borrow_mut().set_dead();
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

/// Struct to hold sprite specific data that both CPU and GPU must access
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

    // flags and params
    z_index: u8,
    draw_flags: DrawFlags,

    pub color: Vector4<f32>,
    pub global_position: Vector2<f32>,
    pub scale: Vector2<f32>,
    image_dimensions: Vector2<u32>,
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
            gl_handler.create_and_bind_texture(texture_path, persistent_set, sampler);

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

        let mut draw_flags = DrawFlags::empty();
        draw_flags.insert(DrawFlags::USED | DrawFlags::VISIBLE);

        Self {
            vertex_buffer,
            descriptor_set,
            cpu_buffer,
            z_index,
            draw_flags,
            color,
            global_position,
            scale,
            image_dimensions,
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
    }

    fn write_flags(&mut self) -> &mut DrawFlags {
        &mut self.draw_flags
    }

    fn read_flags(&self) -> DrawFlags {
        self.draw_flags
    }

    fn set_dead(&mut self) {
        self.draw_flags.remove(DrawFlags::USED);
    }

    fn set_visible(&mut self, visible: bool) {
        self.draw_flags.set(DrawFlags::VISIBLE, visible);
    }
}

/// Struct to handle primitive shapes with simple colours
#[derive(Clone)]
pub struct Primitive {
    vertex_buffer: VertexBuffer,
    descriptor_set: Arc<SpriteImmutableDescriptorSet>,
    cpu_buffer: Arc<CpuAccessibleBuffer<SpriteData>>,

    // flags and params
    z_index: u8,
    draw_flags: DrawFlags,

    pub color: Vector4<f32>,
    pub global_position: Vector2<f32>,
    pub scale: Vector2<f32>,
    image_dimensions: Vector2<u32>,
}

impl Primitive {
    pub fn pixel(texture_path: &str, gl_handler: &GraphicsHandler, z_index: u8) -> Self {
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
            gl_handler.create_and_bind_texture(texture_path, persistent_set, sampler);

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

        let mut draw_flags = DrawFlags::empty();
        draw_flags.insert(DrawFlags::USED | DrawFlags::VISIBLE);

        Self {
            vertex_buffer,
            descriptor_set,
            cpu_buffer,
            z_index,
            draw_flags,
            color,
            global_position,
            scale,
            image_dimensions,
        }
    }
}

impl Draw for Primitive {
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
    }

    fn write_flags(&mut self) -> &mut DrawFlags {
        &mut self.draw_flags
    }

    fn read_flags(&self) -> DrawFlags {
        self.draw_flags
    }

    fn set_dead(&mut self) {
        self.draw_flags.remove(DrawFlags::USED);
    }

    fn set_visible(&mut self, visible: bool) {
        self.draw_flags.set(DrawFlags::VISIBLE, visible);
    }
}

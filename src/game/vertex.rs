use super::GPUWrite;

#[derive(Debug, Clone)]
#[repr(C, align(16))]
pub struct Vertex {
    pub position: glam::Vec3,
    pub color: glam::Vec3,
    pub tex_coord: glam::Vec2,
}

impl GPUWrite for Vertex {}

impl Vertex {
    pub fn binding_description() -> ash::vk::VertexInputBindingDescription {
        ash::vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(std::mem::size_of::<Self>() as u32)
            .input_rate(ash::vk::VertexInputRate::VERTEX)
    }

    pub fn attribute_descriptions() -> [ash::vk::VertexInputAttributeDescription; 3] {
        // color formats are also used to define non-color vec sizes 1-4
        //   (the official tutorial is mildly apologetic)
        // BUT this does matter for defaults -
        //   if there aren't enough components here to fill the components shader-side,
        //   the 'color' components default to 0 and 'alpha' component defaults to 1
        let vec_3_format = ash::vk::Format::R32G32B32_SFLOAT;
        let vec_2_format = ash::vk::Format::R32G32_SFLOAT;

        [
            // position
            ash::vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(0)
                .format(vec_3_format)
                .offset(std::mem::offset_of!(Vertex, position) as u32),
            // color
            ash::vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(1)
                .format(vec_3_format)
                .offset(std::mem::offset_of!(Vertex, color) as u32),
            // texture coordinate
            ash::vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(2)
                .format(vec_2_format)
                .offset(std::mem::offset_of!(Vertex, tex_coord) as u32),
        ]
    }
}

use crate::renderer::vertex_description::VertexDescription;

pub use crate::generated::shader_atlas::depth_texture::Vertex;

impl VertexDescription for Vertex {
    fn binding_descriptions() -> Vec<ash::vk::VertexInputBindingDescription> {
        let binding_description = ash::vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(std::mem::size_of::<Self>() as u32)
            .input_rate(ash::vk::VertexInputRate::VERTEX);

        vec![binding_description]
    }

    fn attribute_descriptions() -> Vec<ash::vk::VertexInputAttributeDescription> {
        // color formats are also used to define non-color vec sizes 1-4
        //   (the official tutorial is mildly apologetic)
        // BUT this does matter for defaults -
        //   if there aren't enough components here to fill the components shader-side,
        //   the 'color' components default to 0 and 'alpha' component defaults to 1
        let glam_vec_3_format = ash::vk::Format::R32G32B32_SFLOAT;
        let glam_vec_2_format = ash::vk::Format::R32G32_SFLOAT;

        // the bindings and locations here are about how this data is read
        // out of the vertex buffer, not the binding to the shader
        // bindings - the index in the array passed to cmd_bind_vertex_buffers
        //   in our case, always 0 because there is only one
        // locations - a unique identifier for the attribute
        vec![
            // position
            ash::vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(0)
                .format(glam_vec_3_format)
                .offset(std::mem::offset_of!(Vertex, position) as u32),
            // color
            ash::vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(1)
                .format(glam_vec_3_format)
                .offset(std::mem::offset_of!(Vertex, color) as u32),
            // texture coordinate
            ash::vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(2)
                .format(glam_vec_2_format)
                .offset(std::mem::offset_of!(Vertex, tex_coord) as u32),
        ]
    }
}

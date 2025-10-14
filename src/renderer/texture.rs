use ash::vk;

pub struct Texture {
    pub(super) image: vk::Image,
    pub(super) image_memory: vk::DeviceMemory,
    pub(super) image_view: vk::ImageView,
    pub(super) sampler: vk::Sampler,
    #[expect(unused)] // currently unused after init
    pub(super) mip_levels: u32,
}

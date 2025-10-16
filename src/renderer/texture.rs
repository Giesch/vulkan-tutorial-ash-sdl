use ash::vk;

#[derive(Debug)]
pub struct TextureHandle(usize);

pub struct TextureStorage(Vec<Option<Texture>>);

// TODO docs w/ panics sections
// maybe extract a generic version
impl TextureStorage {
    pub fn new() -> Self {
        Self(Default::default())
    }

    pub fn add(&mut self, texture: Texture) -> TextureHandle {
        let handle = TextureHandle(self.0.len());
        self.0.push(Some(texture));

        handle
    }

    pub fn get(&self, handle: &TextureHandle) -> &Texture {
        self.0[handle.0].as_ref().unwrap()
    }

    pub fn take(&mut self, handle: TextureHandle) -> Texture {
        self.0[handle.0].take().unwrap()
    }
}

pub struct Texture {
    pub(super) image: vk::Image,
    pub(super) image_memory: vk::DeviceMemory,
    pub(super) image_view: vk::ImageView,
    pub(super) sampler: vk::Sampler,
    #[expect(unused)] // currently unused after init
    pub(super) mip_levels: u32,
}

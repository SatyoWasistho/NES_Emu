use gl::types::*;


pub struct Texture {
    pub id: GLuint,
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteTextures(1, [self.id].as_ptr());
        }
    }
}

impl Texture {
    pub unsafe fn new() -> Self {
        let mut id: GLuint = 0;
        gl::GenTextures(1, &mut id);
        Self { id }
    }

    #[inline(always)]
    pub unsafe fn load(&self, img: &[u8]) {
        self.bind();

        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGBA as i32,
            256,
            240,
            0,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            img.as_ptr() as *const _,
        );
        gl::GenerateMipmap(gl::TEXTURE_2D);
    }

    pub unsafe fn bind(&self) {
        gl::BindTexture(gl::TEXTURE_2D, self.id);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER,gl::NEAREST as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER,gl::NEAREST as i32);
    }
}
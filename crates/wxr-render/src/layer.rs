//! A layer's picture, which is not a frame's eye.
//!
//! Its own file because it is its own shape: the frame loop draws a view into its image, and this draws one
//! callback into an image the compositor placed. The two share a renderer and nothing else.

use super::*;

impl Renderer {
    /// Draw a layer's picture, once, into the image the compositor handed over.
    ///
    /// A layer is not an eye and not a view: the compositor places it and warps it for the optics, which is the
    /// whole reason to hand it one - so it is drawn once, and there is no [`wxr::View`] to draw it with. What
    /// decides how it is seen is the content, which is why the drawing is a callback and not a scene, and what
    /// is here is the part every layer shares: the image, the attachments, and what an empty one is worth.
    ///
    /// `false` is a session that handed over an image this renderer cannot wrap, which is a frame with no
    /// layer rather than a frame to fail - the same answer the eyes get. An image that is not exactly one array
    /// layer is also `false`: it is several pictures, not one, and one callback has nothing to say about which
    /// face or eye it is drawing. A cube is six passes and a layered stereo image is two, and the caller that
    /// has one draws them itself.
    ///
    /// A viewport with no pixels, or one reaching outside the image, is `false` for the same reason the
    /// layered image is: a pass with such a viewport is a validation error rather than a blank layer.
    pub fn draw_layer<I: Import>(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        importer: &I,
        image: &I::Image,
        layer: wxr::LayerImage,
        draw: impl FnOnce(&mut wgpu::RenderPass<'_>),
    ) -> bool {
        // A default view of a layered texture covers every layer, and a render pass cannot attach that as a
        // two-dimensional target. Refusing it is the only answer that does not silently draw one face of six
        // and leave the rest as whatever the compositor had. A viewport that is empty or reaches outside the
        // image is refused for the same reason: there is no valid rectangle to draw in.
        if layer.meta.layers != 1 || !viewport_fits(layer.viewport, layer.meta.extent) {
            return false;
        }
        // The format and the extent are the layer's, which is what a render target is made from - so there is
        // nothing else to pass: an `ImageMeta` beside them would be the same fact twice, and one of the two
        // could be wrong.
        let Some(texture) = importer.texture(device, layer.meta, image) else {
            return false;
        };
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("wxr layer"),
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("wxr layer"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    // Cleared and not loaded: the compositor's image is the compositor's, and nothing here knows
                    // what is in it. A panel's empty parts are nothing, which is what `Renderer::new`'s colour
                    // says - and the alpha in it is why a layer has to be presented with source alpha.
                    load: wgpu::LoadOp::Clear(self.clear),
                    store: wgpu::StoreOp::Store,
                },
            })],
            // No depth: a layer is one picture with nothing behind it, and the compositor has the world for
            // that.
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            ..Default::default()
        });
        // The part of the image that is this layer's, which is all of it on a platform that gives a layer its
        // own texture and a sub-rectangle on one that pools them.
        pass.set_viewport(
            layer.viewport.x as f32,
            layer.viewport.y as f32,
            layer.viewport.width as f32,
            layer.viewport.height as f32,
            0.0,
            1.0,
        );
        draw(&mut pass);
        drop(pass);
        queue.submit(Some(encoder.finish()));
        true
    }
}

//! What the session hands over, and what a run asserts about it.
//!
//! The whole image path in one place: an `ImageMeta` read off the texture the browser gave, one image, one
//! viewport and one array layer per eye, and the room's depth for a session that asked for it. It lives beside
//! the page rather than in it because it is a set of claims about the backend rather than a thing the page
//! does - `tick` asks and this decides whether the answer was the one that was promised.
//!
//! A run without the binding is asserted here too, and not merely survived: "no pictures" is an answer this
//! backend gives as plainly as pictures are, and a page that shrugged at it would never notice the day it
//! stopped being true.

use super::*;

pub(super) fn check_images(board: &mut Board) -> Result<(), String> {
    let meta = board.session.images();
    let count = board.session.image_count();
    say(&format!(
        "images: {count}, {:?} at {:?}, {} array layer(s)",
        meta.format, meta.extent, meta.layers
    ));

    if !board.gpu {
        // What a session with no binding has to say, and the reason `ImageMeta` is read off the texture
        // rather than remembered from the layer's configuration: an unknown format and a zero extent are the
        // honest answer, and a plausible size would be a renderer told to draw a picture that is not there.
        if count != 0 {
            return Err(format!(
                "a session with no binding handed over {count} image(s)"
            ));
        }
        if meta.format != wxr::ColorFormat::Unknown || meta.extent != wxr::Extent2d::default() {
            return Err(format!(
                "a session with no binding described a picture: {:?} at {:?}",
                meta.format, meta.extent
            ));
        }
        if board.session.depth_sensing().is_some() {
            return Err("a session with no binding claims depth sensing it cannot measure".into());
        }
        return Ok(());
    }

    if count != 1 {
        return Err(format!(
            "a projection layer hands over one image for both eyes, and this one handed {count}"
        ));
    }
    if board.session.image(0).is_none() {
        return Err("image 0 is counted but not there".into());
    }
    if meta.format == wxr::ColorFormat::Unknown {
        return Err("the texture's format is one this core has not learned".into());
    }
    if meta.extent.width == 0 || meta.extent.height == 0 {
        return Err(format!("the texture has an extent of {:?}", meta.extent));
    }
    // Two eyes in two array layers, which is what a stereo projection layer is and what `ImageMeta::layers`
    // is there to say. One layer with two side-by-side viewports is the other shape a compositor can have,
    // and it is not this one.
    if meta.layers != 2 {
        return Err(format!(
            "two eyes are two array layers, and the texture says {}",
            meta.layers
        ));
    }

    let whole = wxr::Viewport {
        x: 0,
        y: 0,
        width: meta.extent.width,
        height: meta.extent.height,
    };
    let views = board.frame.views();
    if views.len() != 2 {
        return Err(format!(
            "a stereo session has two views, and this frame has {}",
            views.len()
        ));
    }
    for view in views {
        if view.image != 0 {
            return Err(format!(
                "{:?} eye draws into image {}",
                view.eye, view.image
            ));
        }
        // The viewport is the whole of the layer the eye is in, because the eyes are told apart by the array
        // layer and not by halves of a picture. A viewport left at the default would be 0x0, and a renderer
        // drawing into it would draw nothing and say nothing about why.
        if view.viewport != whole {
            return Err(format!(
                "{:?} eye's viewport is {:?} and the layer it is in is {:?}",
                view.eye, view.viewport, meta.extent
            ));
        }
    }
    if views[0].layer != 0 || views[1].layer != 1 {
        return Err(format!(
            "the eyes are array layers 0 and 1, and this frame says {} and {}",
            views[0].layer, views[1].layer
        ));
    }

    // Depth, which is the other half of what a binding hands over and the reason this run is immersive-ar:
    // `depth-sensing` is asked for there and nowhere else, so this is the only run that can have it.
    let features = board.session.features();
    if !features.contains(wxr::Features::DEPTH) {
        return Err("the session asked for depth-sensing and the DEPTH bit is not set".into());
    }
    let Some(sensing) = board.session.depth_sensing() else {
        return Err("the DEPTH bit is set and depth sensing says nothing".into());
    };
    say(&format!(
        "depth sensing: {:?}, {:?}, {:?}, active {}",
        sensing.usage,
        sensing.format,
        sensing.ty,
        sensing.active.unwrap_or(false)
    ));
    let Some((_, info)) = board.session.depth(0) else {
        return Err("a session configured for GPU depth gave no depth for view 0".into());
    };
    if info.size != meta.extent {
        return Err(format!(
            "the measured depth is {:?} and the colour it lines up with is {:?}",
            info.size, meta.extent
        ));
    }
    // `float32` is metres already, so a scale that is not one is a scale somebody made up - and this is
    // WebXR's own buffer, which is the linear encoding: anything else here would be a session that measured
    // the room and would not say what its numbers meant, which is what `Unknown` is for.
    let to_meters = match info.encoding {
        wxr::DepthEncoding::Linear { to_meters } => to_meters,
        encoding => {
            return Err(format!(
                "float32 depth came back as {encoding:?}, and a browser's is a linear buffer"
            ));
        }
    };
    if (to_meters - 1.0).abs() > 1e-6 {
        return Err(format!("{to_meters} m per raw unit, and float32 is metres"));
    }
    say(&format!(
        "depth: {:?} at {to_meters} m per raw unit, from {:?}",
        info.size, info.norm_depth_buffer_from_norm_view.position,
    ));
    Ok(())
}

//! The pictures the compositor places, which are the half of a layer that is not drawn per eye.
//!
//! Every shape the core has is made here, in the one window a browser allows it: between animation frames.
//! `updateRenderState` throws while a frame is active, and handing over a layer list is what `Session::layer`
//! does - so the run that made layers from inside its frame loop would be a run whose layers were never
//! presented. They are made from the timer that waits for the session instead, which is outside every frame by
//! construction, and read back from inside one.
//!
//! Kept out of `lib.rs` only to stay under the file-size limit; everything it uses comes from there.

use super::*;
use wxr_webxr::WebXrSession;

/// The field a layer's record in the stub has, if it has it.
fn said(index: usize, field: &str) -> Option<JsValue> {
    let said =
        js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("__wxrLayerSaid")).ok()?;
    let said: js_sys::Array = said.dyn_into().ok()?;
    js_sys::Reflect::get(&said.get(index as u32), &JsValue::from_str(field)).ok()
}

fn said_number(index: usize, field: &str) -> Option<f64> {
    said(index, field)?.as_f64()
}

fn said_flag(index: usize, field: &str) -> bool {
    said(index, field).and_then(|value| value.as_bool()) == Some(true)
}

/// Whether the stub's own record says the layer has `expected` in `field`.
fn number(index: usize, field: &str, expected: f32) -> Result<(), String> {
    match said_number(index, field) {
        Some(value) if value == f64::from(expected) => Ok(()),
        other => Err(format!(
            "layer {index}'s {field} is {other:?}, where {expected} was asked for"
        )),
    }
}

/// Make every shape the core has, once the floor space has resolved.
///
/// `Ok(false)` is the space still being a promise: a layer asked for in a space the browser has not answered
/// for is one to ask again, which is the same answer `views` gives. Called from the timer and never from the
/// frame loop, because a browser only takes a layer list between frames.
pub(super) fn prepare(board: &mut Board, session: &mut WebXrSession) -> Result<bool, String> {
    if !board.gpu || board.layers_prepared {
        return Ok(true);
    }
    let Some(floor) = board.space else {
        return Ok(false);
    };
    if board.viewer.is_none() {
        board.viewer = Some(
            session
                .space(wxr::SpaceKind::Viewer)
                .map_err(|error| format!("the viewer space: {error}"))?,
        );
    }
    let viewer = board.viewer.expect("just made");

    if !board.layers_refused {
        // The two shapes that are pictures of the whole environment are restricted to a reference space that
        // is not the viewer, because a skybox locked to the head defeats the reprojection it exists for.
        for shape in [
            wxr::LayerShape::Equirect {
                radius: 2.0,
                central_horizontal: 1.5,
                upper_vertical: 0.8,
                lower_vertical: -0.4,
            },
            wxr::LayerShape::Cube,
        ] {
            match session.layer(viewer, shape, wxr::Extent2d::new(64, 64)) {
                Err(wxr::Error::Rejected(_)) => {}
                other => {
                    return Err(format!(
                        "an environment layer in a viewer space answered {other:?}, where Rejected is the \
                         answer"
                    ));
                }
            }
        }
        // A cube's six faces are square, so the extent it is asked for has to be square as well.
        match session.layer(floor, wxr::LayerShape::Cube, wxr::Extent2d::new(64, 32)) {
            Err(wxr::Error::Rejected(_)) => {}
            other => {
                return Err(format!(
                    "a non-square cube layer answered {other:?}, where Rejected is the answer"
                ));
            }
        }
        board.layers_refused = true;
        say("environment layers were refused the viewer's space, and a cube a non-square extent");
    }

    let quad = match session.layer(
        floor,
        wxr::LayerShape::Quad {
            width: 1.0,
            height: 0.5,
        },
        wxr::Extent2d::new(256, 128),
    ) {
        Ok(layer) => layer,
        Err(wxr::Error::Unavailable(_)) => return Ok(false),
        Err(error) => return Err(format!("a quad layer: {error}")),
    };
    let cylinder = session
        .layer(
            floor,
            wxr::LayerShape::Cylinder {
                radius: 1.0,
                central_angle: 1.2,
                aspect: 1.5,
            },
            wxr::Extent2d::new(128, 128),
        )
        .map_err(|error| format!("a cylinder layer: {error}"))?;
    let equirect = session
        .layer(
            floor,
            wxr::LayerShape::Equirect {
                radius: 2.0,
                central_horizontal: 1.5,
                upper_vertical: 0.8,
                lower_vertical: -0.4,
            },
            wxr::Extent2d::new(128, 64),
        )
        .map_err(|error| format!("an equirect layer: {error}"))?;
    let cube = session
        .layer(floor, wxr::LayerShape::Cube, wxr::Extent2d::new(64, 64))
        .map_err(|error| format!("a cube layer: {error}"))?;

    // A pose is relative to the layer's own space. The quad is moved, because a quad has a transform; the cube
    // is turned, because a cube has an orientation and no position - the compositor always draws it around the
    // viewer.
    session
        .set_layer_pose(
            quad,
            wxr::Pose {
                position: wxr::glam::Vec3::new(0.1, 0.2, 0.3),
                orientation: wxr::glam::Quat::IDENTITY,
            },
        )
        .map_err(|error| format!("moving the quad: {error}"))?;
    session
        .set_layer_pose(
            cube,
            wxr::Pose {
                position: wxr::glam::Vec3::ZERO,
                orientation: wxr::glam::Quat::from_rotation_y(0.25),
            },
        )
        .map_err(|error| format!("turning the cube: {error}"))?;

    board.layers = vec![quad, cylinder, equirect, cube];
    board.layers_prepared = true;
    // A layer with nothing drawn into it needs its first picture, and no `redraw` event says so: the query is
    // the only way an app hears it. The stub starts `needsRedraw` true, which is what the specification says.
    if !session.layer_needs_redraw(quad) {
        return Err("a layer nothing has been drawn into says it needs no picture".into());
    }
    say("a quad, a cylinder, an equirect and a cube were handed to the compositor");
    Ok(true)
}

/// The compositor losing a layer's picture, which it says twice: by setting `needsRedraw` again, and through the
/// `redraw` event on the layer. Both are checked, because polling and listening are the two ways to hear it.
pub(super) fn check_redraw(board: &mut Board, session: &mut WebXrSession) -> Result<(), String> {
    if board.redraw_checked {
        return Ok(());
    }
    let Some(layer) = board.redraw_event else {
        return Ok(());
    };
    board.redraw_checked = true;
    let Some(quad) = board.layers.first().copied() else {
        return Err("a redraw event arrived for a run that has no layers".into());
    };
    if layer != quad {
        return Err(format!(
            "the redraw event named {layer:?}, where the compositor lost the quad {quad:?}"
        ));
    }
    if !session.layer_needs_redraw(layer) {
        return Err("a layer the compositor lost says it needs no picture".into());
    }
    say("the compositor lost the quad's picture, said so, and asked for it again");
    Ok(())
}

/// What every layer gave back in the first frame that has one, and that a released one gives back nothing.
pub(super) fn check_frame(board: &mut Board, session: &mut WebXrSession) -> Result<(), String> {
    if !board.gpu
        || !board.layers_prepared
        || board.layers_checked
        || board.frame.views().is_empty()
    {
        return Ok(());
    }
    board.layers_checked = true;
    // The extent asked for, and the number of pictures in it: a cube's one texture holds six square faces.
    let expected = [
        (1, wxr::Extent2d::new(256, 128)),
        (1, wxr::Extent2d::new(128, 128)),
        (1, wxr::Extent2d::new(128, 64)),
        (6, wxr::Extent2d::new(64, 64)),
    ];
    for (index, layer) in board.layers.clone().into_iter().enumerate() {
        let (layers, extent) = expected[index];
        let Some((_, image)) = session.layer_image(layer) else {
            return Err(format!(
                "the layer in slot {index} has no image in a frame that has one"
            ));
        };
        if image.meta.extent != extent || image.meta.layers != layers {
            return Err(format!(
                "layer {index} is {:?} with {} picture(s), where {extent:?} with {layers} was asked for",
                image.meta.extent, image.meta.layers
            ));
        }
        if image.viewport.width != extent.width || image.viewport.height != extent.height {
            return Err(format!(
                "layer {index}'s viewport is {}x{}, where the layer is {}x{}",
                image.viewport.width, image.viewport.height, extent.width, extent.height
            ));
        }
        if session.binding(layer).is_err() {
            return Err(format!(
                "layer {index} is one this session made and has no binding"
            ));
        }
        // The picture is in the layer now, so it no longer needs one: `needsRedraw` is how a compositor says a
        // layer is out of date, and a backend that never passed the false on would ask an app to draw forever.
        if session.layer_needs_redraw(layer) {
            return Err(format!(
                "layer {index} still wants a picture after the frame drew it"
            ));
        }
        if !said_flag(index, "blendTextureSourceAlpha") {
            return Err(format!(
                "layer {index} does not blend its texture's alpha, where a layer is a picture over what is \
                 already there"
            ));
        }
    }
    // The geometry is the shape's, and the stub's record is what the setters landed on. A backend that sent a
    // radius as an aspect would still make a layer; it would just make the wrong picture.
    number(0, "width", 1.0)?;
    number(0, "height", 0.5)?;
    number(1, "radius", 1.0)?;
    number(1, "aspectRatio", 1.5)?;
    number(2, "radius", 2.0)?;
    number(2, "centralHorizontalAngle", 1.5)?;
    number(2, "upperVerticalAngle", 0.8)?;
    number(2, "lowerVerticalAngle", -0.4)?;
    let turn = wxr::glam::Quat::from_rotation_y(0.25);
    match said(3, "orientation").and_then(|point| {
        js_sys::Reflect::get(&point, &JsValue::from_str("y"))
            .ok()
            .and_then(|value| value.as_f64())
    }) {
        Some(value) if value == f64::from(turn.y) => {}
        other => {
            return Err(format!(
                "the cube's orientation is {other:?}, where a quarter turn is {}",
                turn.y
            ));
        }
    }

    // A layer that is gone is a layer the browser is not presenting: no image, and nothing to bind.
    let released = board.layers[1];
    session.release_layer(released);
    if session.layer_image(released).is_some() {
        return Err("a released layer still has an image".into());
    }
    if session.binding(released).is_ok() {
        return Err("a released layer still has a binding".into());
    }
    // A layer handed over from inside a frame is a layer the browser refuses to be told about, and a handle to
    // one would be a handle to a picture nothing composites. The refusal is the answer.
    if let Some(floor) = board.space {
        match session.layer(
            floor,
            wxr::LayerShape::Quad {
                width: 0.5,
                height: 0.5,
            },
            wxr::Extent2d::new(32, 32),
        ) {
            Err(wxr::Error::Rejected(_)) => {}
            other => {
                return Err(format!(
                    "a layer made during an animation frame answered {other:?}, where Rejected is the answer"
                ));
            }
        }
    }
    say(
        "four layers drew once each, with six faces for the cube, and the released one drew no more",
    );
    Ok(())
}

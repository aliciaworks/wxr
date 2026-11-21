//! Hit testing: where a ray out of a space meets the world.
//!
//! `XRHitTestSource` is created asynchronously and is otherwise opaque, so one waits in a slot until the
//! browser answers - and a frame with no source yet is a frame with no hits rather than a frame to fail. A
//! profile source is the same shape with a different browser object: it follows the input sources the wearer
//! actually has, so its hits arrive in groups, one per source.

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use crate::input::Sources;
use crate::sys::{
    DomPointInit, XrFrame, XrHitTestOptionsInit, XrHitTestResult, XrHitTestSource, XrRay,
    XrRayDirectionInit, XrReferenceSpace, XrSession, XrTransientInputHitTestOptionsInit,
    XrTransientInputHitTestResult, XrTransientInputHitTestSource,
};

/// What the browser answered with, which is one of the two kinds of hit-test source it has.
///
/// They are the same handle to a caller - both are cancelled the same way and both answer through `hits` - so
/// they share a slot and an id, and only the ask and the read differ.
#[derive(Clone)]
pub enum Source {
    /// A ray the app described, which is `XRHitTestSource`.
    Ray(XrHitTestSource),
    /// A profile whose matching sources aim themselves, which is `XRTransientInputHitTestSource`.
    Input(XrTransientInputHitTestSource),
}

impl Source {
    /// Tell the browser this source is done, whichever kind it is.
    fn cancel(self) {
        match self {
            Self::Ray(source) => source.cancel(),
            Self::Input(source) => source.cancel(),
        }
    }
}

/// A source that has been asked for, and the browser's answer once it arrives.
#[derive(Default)]
pub struct Slot {
    source: Option<Source>,
    /// Set by [`Slot::cancel`], so an answer arriving afterwards fills nothing. A source let go while its
    /// request was still a promise would otherwise come back from having been released, which is the one
    /// way this slot can be wrong - the handle is still in the caller's hand either way.
    cancelled: bool,
}

impl Slot {
    /// The source, if the browser has answered and nobody has let it go.
    pub fn source(&self) -> Option<Source> {
        self.source.clone()
    }

    /// Tell the browser this source is done - `XRHitTestSource.cancel` or its transient sibling - and forget it.
    ///
    /// Both halves are needed. An answered source would otherwise be one the browser keeps reporting for
    /// the rest of the session, and an unanswered one must not be filled in when its promise lands - so the
    /// flag outlives the promise, which is what makes a release at any moment the same release.
    pub fn cancel(&mut self) {
        self.cancelled = true;
        if let Some(source) = self.source.take() {
            source.cancel();
        }
    }

    /// Take the browser's answer, unless the source was let go while its request was in flight - in which
    /// case it is cancelled here rather than kept, since no caller will ever ask for it.
    pub fn fill(&mut self, source: Source) {
        if self.cancelled {
            source.cancel();
        } else {
            self.source = Some(source);
        }
    }
}

/// Ask the browser for a source aiming out of `space`, along `ray` when the app gave one, and fill the slot
/// when it answers.
///
/// Everything else about the options has a default, so there are three: the space, WebXR's `offsetRay` - a
/// ray in that space's own coordinates, which is the one the app described and [`offset`] already made - and
/// WebXR's `entityTypes`, which is passed every time because the browser's default is `["plane"]` alone and
/// the core's [`wxr::HitTestKinds::ALL`] means all three.
pub fn request(
    session: &XrSession,
    space: &XrReferenceSpace,
    kinds: wxr::HitTestKinds,
    ray: Option<XrRay>,
    slot: Rc<RefCell<Slot>>,
) {
    let options = XrHitTestOptionsInit::new(space);
    options.set_entity_types(&entity_types(kinds));
    if let Some(ray) = &ray {
        options.set_offset_ray(ray);
    }
    let promise: js_sys::Promise = session.request_hit_test_source(&options);
    wasm_bindgen_futures::spawn_local(async move {
        if let Ok(source) = JsFuture::from(promise).await
            && let Ok(source) = source.dyn_into::<XrHitTestSource>()
        {
            slot.borrow_mut().fill(Source::Ray(source));
        }
    });
}

/// Ask the browser for a source that follows whichever input source carries `profile`, which is
/// `requestHitTestSourceForTransientInput`, and fill the slot when it answers.
///
/// No space and no ray: each matching source aims along its own target ray, so there is nothing here to
/// describe one with. `entityTypes` is passed for the same reason a ray source's is - the browser's default
/// is a plane alone and the core's `ALL` is all three.
pub fn request_transient(
    session: &XrSession,
    kinds: wxr::HitTestKinds,
    profile: &str,
    slot: Rc<RefCell<Slot>>,
) {
    let options = XrTransientInputHitTestOptionsInit::new(profile);
    options.set_entity_types(&entity_types(kinds));
    let promise: js_sys::Promise = session.request_hit_test_source_for_transient_input(&options);
    wasm_bindgen_futures::spawn_local(async move {
        if let Ok(source) = JsFuture::from(promise).await
            && let Ok(source) = source.dyn_into::<XrTransientInputHitTestSource>()
        {
            slot.borrow_mut().fill(Source::Input(source));
        }
    });
}

/// The core's kinds as WebXR's `sequence<XRHitTestTrackableType>`.
///
/// An empty request is an empty sequence, which the browser reads as no kinds at all and so reports no hits -
/// which is what [`wxr::HitTestKinds::NONE`] asks for. The three names are the specification's, and a kind the
/// core has no word for cannot be asked for because there is nothing here to name it with.
fn entity_types(kinds: wxr::HitTestKinds) -> JsValue {
    let array = js_sys::Array::new();
    for (kind, name) in [
        (wxr::HitTestKinds::POINTS, "point"),
        (wxr::HitTestKinds::PLANES, "plane"),
        (wxr::HitTestKinds::MESHES, "mesh"),
    ] {
        if kinds.contains(kind) {
            array.push(&JsValue::from_str(name));
        }
    }
    array.into()
}

/// The core's ray as the browser's `XRRay`.
///
/// The two halves are a `DOMPointInit` and an `XRRayDirectionInit`, which is exactly the constructor for an
/// origin and a direction. `XRRay` belongs to the hit test module along with the source, so a browser with one
/// has the other; one that somehow does not is `Unsupported` rather than a source asked for along the space's
/// aim, because the app named a ray and a hit test along a different one is not an answer to it.
pub fn offset(ray: wxr::Ray) -> Result<XrRay, wxr::Error> {
    let origin = DomPointInit::new();
    origin.set_x(f64::from(ray.origin.x));
    origin.set_y(f64::from(ray.origin.y));
    origin.set_z(f64::from(ray.origin.z));
    let direction = XrRayDirectionInit::new();
    direction.set_x(f64::from(ray.direction.x));
    direction.set_y(f64::from(ray.direction.y));
    direction.set_z(f64::from(ray.direction.z));
    XrRay::new_with_dom_point_init_and_direction(&origin, &direction)
        .map_err(|_| wxr::Error::Unsupported("a hit-test ray: this browser has no XRRay".into()))
}

/// Read this frame's hits along `source`, expressed in `base`, into the core's shape.
pub fn results(
    frame: &XrFrame,
    source: &XrHitTestSource,
    base: &XrReferenceSpace,
    out: &mut Vec<wxr::Hit>,
) {
    collect(&frame.get_hit_test_results(source), base, None, out);
}

/// The same for a profile source, which WebXR answers per input source - `XRTransientInputHitTestResult` -
/// rather than as one flat list. Every hit keeps the identity of the pointer that aimed it, under the id
/// `inputs` gives that same source.
pub fn transient_results(
    frame: &XrFrame,
    source: &XrTransientInputHitTestSource,
    base: &XrReferenceSpace,
    sources: &Sources,
    out: &mut Vec<wxr::Hit>,
) {
    for result in frame
        .get_hit_test_results_for_transient_input(source)
        .iter()
    {
        let Ok(result) = result.dyn_into::<XrTransientInputHitTestResult>() else {
            continue;
        };
        // Both halves come from the same table the frame loop reads: a hit that named a number of its own
        // would be a pointer no `inputs` call could find.
        let input = sources.id(&result.input_source());
        collect(&result.results(), base, Some(input), out);
    }
}

/// One list of results as the core's hits, tagged with the source that aimed when a pointer did.
fn collect(
    results: &js_sys::Array,
    base: &XrReferenceSpace,
    input: Option<wxr::InputId>,
    out: &mut Vec<wxr::Hit>,
) {
    // One result per surface the ray meets, and an empty sequence for a ray that meets nothing.
    for result in results.iter() {
        let Ok(result) = result.dyn_into::<XrHitTestResult>() else {
            continue;
        };
        // Where is the whole of what a hit test says. A result the runtime will not answer for is one that
        // missed this frame, which is an answer too.
        if let Some(pose) = result.get_pose(base) {
            out.push(wxr::Hit {
                pose: crate::transform(pose.transform()),
                input,
            });
        }
    }
}

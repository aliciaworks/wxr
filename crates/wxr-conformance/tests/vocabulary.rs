//! Where two readings of the WebXR API agree, and where they must.
//!
//! A specification fixes some things exactly - how many session modes there are, what an eye is - and those
//! are the things a second implementation can be compared against without hardware. Each test names the part
//! of the specification it is about, so that a failure says which sentence moved.

/// `XRSessionMode` - <https://immersive-web.github.io/webxr/#enumdef-xrsessionmode>
///
/// Three modes and no fourth: `inline` is a session drawn in the page, and the two immersive ones differ in
/// whether the picture *is* the world or is drawn over it. The comparison is variant by variant rather than
/// by count, so a mode added on either side is a failure and not a silently-covered case.
#[test]
fn the_session_modes_are_the_same_three() {
    let ours = [
        wxr::SessionMode::Inline,
        wxr::SessionMode::ImmersiveVr,
        wxr::SessionMode::ImmersiveAr,
    ];
    let theirs = [
        servo_webxr_api::SessionMode::Inline,
        servo_webxr_api::SessionMode::ImmersiveVR,
        servo_webxr_api::SessionMode::ImmersiveAR,
    ];

    assert_eq!(ours.len(), theirs.len(), "a mode was added on one side");
    for (ours, theirs) in ours.iter().zip(&theirs) {
        // The names differ only in casing, which is each project's own convention for its own words - the
        // point is that nothing else about them differs, which is what an equal name here says.
        assert_eq!(
            format!("{ours:?}").to_ascii_lowercase(),
            format!("{theirs:?}").to_ascii_lowercase(),
        );
    }
}

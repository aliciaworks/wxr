//! How the headset is sitting on the wearer, which is the display asking about its own optics.

/// Where the wearer's eyes are relative to the range the display is made for - visionOS's
/// `ar_device_fit_status_t`, and a fact no WebXR or OpenXR runtime in this workspace reports.
///
/// It is not a health measurement and not a pose. The optics have a range they are designed around, and a
/// wearer whose eyes drift out of it sees a picture softer or more clipped than the one the app drew; the six
/// directions are the ways out of that range, so an app can say "up" rather than "wrong".
///
/// [`Unknown`](DeviceFit::Unknown) covers both a session that cannot ask - one without
/// [`Features::DEVICE_FIT`](crate::Features::DEVICE_FIT) - and one that has not measured yet, because a hint
/// drawn from no measurement is worse than no hint at all.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum DeviceFit {
    /// No measurement, or a session with no way to make one.
    #[default]
    Unknown,
    /// The eyes are inside the range the display is made for.
    Valid,
    /// The headset is sitting too high: the eyes are above the range.
    EyesAbove,
    /// Too low: the eyes are below the range.
    EyesBelow,
    /// Too far to the wearer's left.
    EyesLeft,
    /// Too far to the wearer's right.
    EyesRight,
    /// Too close to the display: the headset is pushed forward.
    EyesForward,
    /// Too far from the display: the headset sits back.
    EyesBackward,
}

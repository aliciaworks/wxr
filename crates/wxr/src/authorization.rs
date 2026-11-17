//! Whether the platform let a session have what it has.

/// What a platform has decided about one capability's permission - WebXR's `XRPermissionStatus` and ARKit's
/// `ar_authorization_status_t`, which are the same three answers in two spellings.
///
/// [`crate::Session::features`] says what a session *has*; this says whether it was *allowed*, and the two
/// questions ask different things of an app. A capability a platform does not have is not something the
/// wearer can turn on, while one they refused is a sentence that says "change it in Settings" rather than
/// "not on this device" - the difference between a dead end and a way out.
///
/// A platform with no permission model has nothing to refuse and answers [`Granted`](Authorization::Granted)
/// to everything. That is also what a session answers about a capability it does not have, because
/// [`crate::Session::features`] has already said it is not there.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Authorization {
    /// Nobody has been asked yet, so nothing has been refused: the platform puts the question up the first
    /// time the capability is used, and an app that wants to explain an empty room reads this as "not yet".
    NotDetermined,
    /// Allowed - by the wearer, or by a policy that did not need to ask them.
    Granted,
    /// Refused, and asking again does not help: the answer lives in the platform's settings and only the
    /// wearer can change it.
    Denied,
}

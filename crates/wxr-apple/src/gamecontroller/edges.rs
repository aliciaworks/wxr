//! Presses as edges: the state GameController hands out, turned into the events a caller reacts to.

/// Queue the edges between two readings of a controller's buttons.
///
/// The core's events are what a caller reacts to, and GameController hands out a state rather than a
/// notification - so the same comparison OpenXR makes once a frame is made here, with the release before the
/// completed selection, which is the order the core documents and a browser fires.
pub(super) fn press_edges(
    id: wxr::InputId,
    was: wxr::Buttons,
    now: wxr::Buttons,
    out: &mut Vec<wxr::Event>,
) {
    if now.select != was.select {
        out.push(if now.select {
            wxr::Event::SelectStart(id)
        } else {
            wxr::Event::SelectEnd(id)
        });
        if !now.select {
            out.push(wxr::Event::Select(id));
        }
    }
    if now.squeeze != was.squeeze {
        out.push(if now.squeeze {
            wxr::Event::SqueezeStart(id)
        } else {
            wxr::Event::SqueezeEnd(id)
        });
        if !now.squeeze {
            out.push(wxr::Event::Squeeze(id));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gamecontroller::CONTROLLER_ID_BASE;

    /// A press is the two edges and the completed selection, in the order a caller reads them.
    #[test]
    fn a_press_is_a_start_an_end_and_a_selection() {
        let id = wxr::InputId::new(CONTROLLER_ID_BASE);
        let mut out = Vec::new();
        press_edges(
            id,
            wxr::Buttons::default(),
            wxr::Buttons {
                select: true,
                ..Default::default()
            },
            &mut out,
        );
        assert_eq!(out, [wxr::Event::SelectStart(id)]);
        out.clear();
        press_edges(
            id,
            wxr::Buttons {
                select: true,
                ..Default::default()
            },
            wxr::Buttons::default(),
            &mut out,
        );
        assert_eq!(
            out,
            [wxr::Event::SelectEnd(id), wxr::Event::Select(id)],
            "the release and then the selection it completes"
        );
    }

    /// The same for the grip, which is a separate pair of edges.
    #[test]
    fn a_squeeze_is_a_start_an_end_and_a_squeeze() {
        let id = wxr::InputId::new(CONTROLLER_ID_BASE);
        let mut out = Vec::new();
        press_edges(
            id,
            wxr::Buttons::default(),
            wxr::Buttons {
                squeeze: true,
                ..Default::default()
            },
            &mut out,
        );
        assert_eq!(out, [wxr::Event::SqueezeStart(id)]);
        out.clear();
        press_edges(
            id,
            wxr::Buttons {
                squeeze: true,
                ..Default::default()
            },
            wxr::Buttons::default(),
            &mut out,
        );
        assert_eq!(out, [wxr::Event::SqueezeEnd(id), wxr::Event::Squeeze(id)]);
    }
}

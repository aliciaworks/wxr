dictionary XRTrackedImageInit {
  required ImageBitmap image;
  required float widthInMeters;
};

partial dictionary XRSessionInit {
  sequence<XRTrackedImageInit> trackedImages;
};

enum XRImageTrackability {
  "untrackable",
  "trackable",
};

partial interface XRSession {
  Promise<FrozenArray<XRImageTrackability>> getImageTrackability();
};

enum XRImageTrackingState {
  "untracked",
  "tracked",
  "emulated",
};

[SecureContext, Exposed=Window]
interface XRImageTrackingResult {
  [SameObject] readonly attribute XRSpace imageSpace;
  readonly attribute unsigned long index;
  readonly attribute XRImageTrackingState trackingState;
  readonly attribute float measuredWidthInMeters;
};

partial interface XRFrame {
  FrozenArray<XRImageTrackingResult> getImageTrackingResults();
};

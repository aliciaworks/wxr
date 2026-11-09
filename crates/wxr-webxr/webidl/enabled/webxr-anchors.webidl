[SecureContext, Exposed=Window]
interface XRAnchor {
  readonly attribute XRSpace anchorSpace;

  Promise<DOMString> requestPersistentHandle();

  undefined delete();
};

partial interface XRFrame {
  Promise<XRAnchor> createAnchor(XRRigidTransform pose, XRSpace space);
};

partial interface XRSession {
  readonly attribute FrozenArray<DOMString> persistentAnchors;

  Promise<XRAnchor> restorePersistentAnchor(DOMString uuid);
  Promise<undefined> deletePersistentAnchor(DOMString uuid);
};

partial interface XRHitTestResult {
  Promise<XRAnchor> createAnchor();
};

[Exposed=Window]
interface XRAnchorSet {
  readonly setlike<XRAnchor>;
};

partial interface XRFrame {
  [SameObject] readonly attribute XRAnchorSet trackedAnchors;
};

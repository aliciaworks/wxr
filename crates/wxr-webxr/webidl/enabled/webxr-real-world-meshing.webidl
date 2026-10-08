[Exposed=Window] interface XRMesh {
    [SameObject] readonly attribute XRSpace meshSpace;

    readonly attribute Float32Array vertices;
    readonly attribute Uint32Array indices;
    readonly attribute DOMHighResTimeStamp lastChangedTime;
    readonly attribute DOMString? semanticLabel;
};

[Exposed=Window] interface XRMeshSet {
  readonly setlike<XRMesh>;
};

partial interface XRFrame {
  readonly attribute XRMeshSet detectedMeshes;
};

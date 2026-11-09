partial interface XRView {
  [SameObject] readonly attribute XRCamera? camera;
};

[SecureContext, Exposed=Window]
interface XRCamera {
  readonly attribute unsigned long width;
  readonly attribute unsigned long height;
};

partial interface XRWebGLBinding {
  WebGLTexture? getCameraImage(XRCamera camera);
};

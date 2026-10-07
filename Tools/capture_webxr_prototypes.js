// Capture a browser's WebXR prototypes, for `Tools/check_webxr_sys.py` to check the generated bindings against.
//
// Run it in the console of any page in a secure context (`about:blank` is not one, so `navigator.xr` and the
// whole WebXR namespace are absent there) and it prints the JSON that belongs in
// `crates/wxr-webxr/webidl/prototypes.json`.
//
// It is here because the IDL is not the truth about what a browser implements, and because the two ways this
// backend can name something wrong are both invisible to a compiler: a name the generator took from the IDL, and
// a name written by hand in `throws.rs`. What the generated module *cannot* get wrong is a spelling, since it
// has none of its own - so a name that is here in the IDL and absent from a browser is usually the
// specification being ahead of the implementation, which is worth knowing and not worth failing over.
//
// Members inherited from the DOM's `EventTarget` are dropped: they are the same on everything that inherits
// them, they are not WebXR's to promise, and they are a third of the file.
(() => {
  const dom = new Set([
    "addEventListener", "removeEventListener", "dispatchEvent", "when",
    "bubbles", "cancelBubble", "cancelable", "composed", "composedPath", "currentTarget", "defaultPrevented",
    "eventPhase", "initEvent", "preventDefault", "returnValue", "srcElement", "stopImmediatePropagation",
    "stopPropagation", "target", "timeStamp", "type",
    "AT_TARGET", "BUBBLING_PHASE", "CAPTURING_PHASE", "NONE",
  ]);
  const out = {};
  for (const name of Object.getOwnPropertyNames(globalThis)) {
    if (!/^XR/.test(name)) continue;
    const constructor = globalThis[name];
    if (typeof constructor !== "function" || !constructor.prototype) continue;
    const members = new Set();
    for (let proto = constructor.prototype; proto && proto !== Object.prototype; proto = Object.getPrototypeOf(proto)) {
      for (const key of Object.getOwnPropertyNames(proto)) {
        if (key !== "constructor" && !dom.has(key)) members.add(key);
      }
    }
    out[name] = [...members].sort();
  }
  console.log(JSON.stringify(out, null, 2));
})();

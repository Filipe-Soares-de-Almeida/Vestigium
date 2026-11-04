/** Whether Ctrl is held down in the window; losing focus counts as letting go. */
export class CtrlHold {
  held = $state(false);

  /** Starts listening; the returned function stops. */
  listen(): () => void {
    const press = (event: KeyboardEvent): void => {
      if (event.key === "Control") this.held = true;
    };
    const lift = (event: KeyboardEvent): void => {
      if (event.key === "Control" || !event.ctrlKey) this.held = false;
    };
    const drop = (): void => {
      this.held = false;
    };
    const hidden = (): void => {
      if (document.visibilityState !== "visible") this.held = false;
    };
    // A key-up the window never saw shows up as a pointer without Ctrl.
    const pointer = (event: PointerEvent): void => {
      if (this.held && !event.ctrlKey) this.held = false;
    };
    window.addEventListener("keydown", press);
    window.addEventListener("keyup", lift);
    window.addEventListener("blur", drop);
    window.addEventListener("pointermove", pointer);
    document.addEventListener("visibilitychange", hidden);
    return () => {
      window.removeEventListener("keydown", press);
      window.removeEventListener("keyup", lift);
      window.removeEventListener("blur", drop);
      window.removeEventListener("pointermove", pointer);
      document.removeEventListener("visibilitychange", hidden);
      this.held = false;
    };
  }
}

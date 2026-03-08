/**
 * Manages the full-screen visual overlay shown during the ceremony.
 *
 * The overlay is injected once into `document.body` and toggled with CSS
 * classes.  It intentionally stays on top of all other content to provide an
 * unmissable visual indicator.
 */

export class OverlayController {
  private el: HTMLElement;

  constructor() {
    this.el = document.createElement("div");
    this.el.id = "ceremonyOverlay";
    this.el.className = "overlay overlay--hidden";
    this.el.setAttribute("aria-live", "assertive");
    this.el.setAttribute("role", "alert");

    this.el.innerHTML = `
      <div class="overlay__inner">
        <div class="overlay__cross" aria-hidden="true"></div>
        <p class="overlay__text">ХВИЛИНА<br/>МОВЧАННЯ</p>
        <p class="overlay__sub">09:00</p>
      </div>
    `;

    document.body.appendChild(this.el);
  }

  show(): void {
    this.el.classList.remove("overlay--hidden");
    this.el.classList.add("overlay--visible");
  }

  hide(): void {
    this.el.classList.add("overlay--hidden");
    this.el.classList.remove("overlay--visible");
  }

  get isVisible(): boolean {
    return this.el.classList.contains("overlay--visible");
  }
}

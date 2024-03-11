import htmx from "htmx.org";

console.log(`htmx ${htmx.version}`);
window.htmx = htmx;

htmx.onLoad(() => {
  for (const card of htmx.findAll(
    "[data-flashcard]:not([data-callback-added])",
  ) as NodeListOf<HTMLElement>) {
    card.addEventListener("mouseenter", (_) => {
      const bounds = card.getBoundingClientRect();
      const halfWidth = bounds.width / 2;
      const halfHeight = bounds.height / 2;

      function listener(event: MouseEvent) {
        const x = -((event.clientX - bounds.x - halfWidth) / halfWidth) + 0.5;
        const y = (event.clientY - bounds.y - halfHeight) / halfHeight + 0.5;

        card.style.setProperty("--x", `${x * 100}%`);
        card.style.setProperty("--y", `${y * 100}%`);
      }

      card.addEventListener("mousemove", listener);
      card.addEventListener("mouseleave", (_) => {
        card.removeEventListener("mousemove", listener);
      });
    });
    card.dataset.callbackAdded = "true";
  }
});

function wordify(element: HTMLElement, wordNumber: number = 0): number {
  if (element.dataset.wordified || element.classList.contains("katex")) {
    return wordNumber;
  }

  element.dataset.wordified = "true";

  for (const child of element.childNodes) {
    if (child instanceof HTMLElement) {
      wordNumber == wordify(child, wordNumber);
    }

    if (child instanceof Text) {
      const template = document.createElement("template");
      const html =
        child.textContent?.replace(
          /[^\d\W]+/gm,
          (match) =>
            `<span class="word" data-wordified="true" style="--word-number:${wordNumber++}">${match}</span>`,
        ) || "";
      template.innerHTML = html;
      child.replaceWith(template.content);
    }
  }

  return wordNumber;
}

window.wordify = wordify;

declare global {
  interface Window {
    wordify: typeof wordify;
    htmx: typeof htmx;
  }
}

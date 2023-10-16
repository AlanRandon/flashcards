import "htmx.org";

function wordify(element: HTMLElement) {
	if (element.dataset.wordified) {
		return;
	}

	element.dataset.wordified = "true";

	for (const child of element.childNodes) {
		if (child instanceof HTMLElement) {
			wordify(child);
		}

		if (child instanceof Text) {
			const template = document.createElement("template");
			const html =
				child.textContent?.replace(
					/[^\s]+/gm,
					`<span class="word">$&</span>`,
				) || "";
			template.innerHTML = html;
			child.replaceWith(template.content);
		}
	}
}

window.wordify = wordify;

declare global {
	interface Window {
		wordify: typeof wordify;
	}
}

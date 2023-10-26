import htmx from "htmx.org";

console.log(`htmx ${htmx.version}`);
window.htmx = htmx;

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

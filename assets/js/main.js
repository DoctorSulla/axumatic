document.addEventListener("DOMContentLoaded", function () {
	const forms = document.querySelectorAll("form");
	for (const form of forms) {
		form.addEventListener("submit", async function (e) {
			e.preventDefault();
			const action = this.action;
			const method = this.method.toUpperCase();
			const payload = {};
			const fields = this.querySelectorAll("input");
			for (const field of fields) {
				payload[field.id] = field.value;
			}
			const options = {};
			options.body = JSON.stringify(payload);
			options.method = method;
			options.headers = {
				"Content-Type": "application/json",
			};
			try {
				const response = await fetch(action, options);
				handleResponse(response);
			} catch (e) {
				console.error(e);
			}
		});
	}
});

async function handleResponse(response) {
	if (response.ok) {
		const json = await response.json();
		console.log(json);
	}
}

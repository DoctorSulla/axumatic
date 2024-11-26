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

      const response = await fetch(action, options);
      handleResponse(response);
    });
  }
});

async function handleResponse(response) {
  if (response.status == 401) {
    window.location.assign("/assets/login.html");
  }
  const json = await response.json();
  switch (json.response_type) {
    case "Error":
      document.querySelector("#error").innerText = json.message;
      break;
    case "RegistrationSuccess":
      window.location.assign("/assets/login.html");
      break;
    case "LoginSuccess":
      window.location.assign("/assets/index.html");
      break;
  }
}

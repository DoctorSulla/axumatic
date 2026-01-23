<h1 class="my-2 text-3xl text-blue-500">Welcome to Axumatic</h1>
<p class="py-2 text-lg">
	This is the reference app for Axumatic. Axumatic is built on the popular Rust framework <a
		class="text-blue-500 underline"
		href="https://docs.rs/axum/latest/axum/">Axum</a
	> and provides functionality such as database connectivity, auth and email sending with SMTP which
	I found myself setting up again and again on new Rust projects.
</p>

<p class="py-2 text-lg">
	It should be possible to use any PostgreSQL database and any email provider which has SMTP
	support. The primary method of registration and login is using a user provided email and password,
	although there is also rudimentary support for Google as an identity provider.
</p>

<p class="py-2 text-lg">
	The source code for the Rust backend can be found <a
		class="text-blue-500 underline"
		href="https://github.com/DoctorSulla/axumatic">here</a
	>
	and for the Svelte frontend
	<a class="text-blue-500 underline" href="https://github.com/DoctorSulla/axumatic-reference-app"
		>here</a
	>.
</p>

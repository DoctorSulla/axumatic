<script lang="ts">
	import { goto } from '$app/navigation';
	import { api } from '$lib/api';
	import GoogleSignIn from '$lib/GoogleSignIn.svelte';

	let email = '';
	let username = '';
	let password = '';
	let confirmPassword = '';
	let loading = false;
	let error = '';

	async function handleRegister() {
		loading = true;
		let result = await api.register({
			username: username,
			email: email,
			password: password,
			confirm_password: confirmPassword
		});

		if (result.response_type == 'Error') {
			error = result.message;
		} else {
			error = '';
			goto('/login');
		}
		loading = false;
	}
</script>

<div class="flex min-h-screen items-center justify-center bg-gray-50 px-4 py-12 sm:px-6 lg:px-8">
	<div class="w-full max-w-md space-y-8">
		<div>
			<h2 class="mt-6 text-center text-3xl font-extrabold text-gray-900">Create your account</h2>
			<p class="mt-2 text-center text-sm text-gray-600">
				Or
				<a href="/login" class="font-medium text-indigo-600 hover:text-indigo-500">
					sign in to your existing account
				</a>
			</p>
		</div>
		<form class="mt-8 space-y-6" on:submit|preventDefault={handleRegister}>
			<div class="space-y-4">
				<div>
					<label for="email" class="block text-sm font-medium text-gray-700"> Email address </label>
					<input
						id="email"
						name="email"
						type="email"
						autocomplete="email"
						required
						class="relative mt-1 block w-full appearance-none rounded-md border border-gray-300 px-3 py-2 text-gray-900 placeholder-gray-500 focus:z-10 focus:border-indigo-500 focus:ring-indigo-500 focus:outline-none sm:text-sm"
						placeholder="Email address"
						bind:value={email}
					/>
				</div>
				<div>
					<label for="username" class="block text-sm font-medium text-gray-700"> Username </label>
					<input
						id="username"
						name="username"
						type="text"
						autocomplete="username"
						required
						class="relative mt-1 block w-full appearance-none rounded-md border border-gray-300 px-3 py-2 text-gray-900 placeholder-gray-500 focus:z-10 focus:border-indigo-500 focus:ring-indigo-500 focus:outline-none sm:text-sm"
						placeholder="Username"
						bind:value={username}
					/>
				</div>
				<div>
					<label for="password" class="block text-sm font-medium text-gray-700"> Password </label>
					<input
						id="password"
						name="password"
						type="password"
						autocomplete="new-password"
						required
						class="relative mt-1 block w-full appearance-none rounded-md border border-gray-300 px-3 py-2 text-gray-900 placeholder-gray-500 focus:z-10 focus:border-indigo-500 focus:ring-indigo-500 focus:outline-none sm:text-sm"
						placeholder="Password"
						bind:value={password}
					/>
				</div>
				<div>
					<label for="confirmPassword" class="block text-sm font-medium text-gray-700">
						Confirm Password
					</label>
					<input
						id="confirmPassword"
						name="confirmPassword"
						type="password"
						autocomplete="new-password"
						required
						class="relative mt-1 block w-full appearance-none rounded-md border border-gray-300 px-3 py-2 text-gray-900 placeholder-gray-500 focus:z-10 focus:border-indigo-500 focus:ring-indigo-500 focus:outline-none sm:text-sm"
						placeholder="Confirm Password"
						bind:value={confirmPassword}
					/>
				</div>
			</div>

			{#if error}
				<div class="text-center text-sm text-red-600">{error}</div>
			{/if}

			<div>
				<button
					type="submit"
					disabled={loading}
					class="group relative flex w-full justify-center rounded-md border border-transparent bg-indigo-600 px-4 py-2 text-sm font-medium text-white hover:bg-indigo-700 focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 focus:outline-none disabled:cursor-not-allowed disabled:opacity-50"
				>
					{loading ? 'Creating account...' : 'Create account'}
				</button>
			</div>
		</form>

		<GoogleSignIn />
	</div>
</div>

<script context="module" lang="ts">
	declare const google: any;
</script>

<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { goto } from '$app/navigation';

	let error = '';
	let googleButtonWrapper: HTMLElement;

	onMount(async () => {
		let response = await api.getNonce();
		let nonce = response.message;
		const script = document.createElement('script');
		script.src = 'https://accounts.google.com/gsi/client';
		script.async = true;
		script.defer = true;
		script.onload = () => {
			google.accounts.id.initialize({
				client_id: '988343938519-vle7kps2l5f6cdnjluibda25o66h2jpn.apps.googleusercontent.com',
				callback: handleCredentialResponse,
				nonce: nonce
			});
			google.accounts.id.renderButton(
				googleButtonWrapper,
				{ theme: 'outline', size: 'large' } // Customization options
			);
			google.accounts.id.prompt(); // Also display the One Tap prompt
		};
		document.head.appendChild(script);
	});

	async function handleCredentialResponse(response: any) {
		let request = await api.googleLogin({ jwt: response.credential });
		if (request.response_type == 'Error') {
			error = request.message;
		} else {
			goto('/profile');
		}
	}
</script>

<div bind:this={googleButtonWrapper}></div>
{#if error}
	<div class="text-center text-sm text-red-600">{error}</div>
{/if}

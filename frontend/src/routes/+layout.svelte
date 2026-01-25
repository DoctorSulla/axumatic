<script lang="ts">
	import '../app.css';
	import favicon from '$lib/assets/favicon.svg';
	import { page } from '$app/state';
	import { afterNavigate } from '$app/navigation';
	import { api } from '$lib/api';
	import { goto } from '$app/navigation';

	let { children } = $props();

	interface Route {
		text: string;
		href: string;
		allowed: 'always' | 'authenticated' | 'unauthenticated';
	}

	let links: Route[] = $state([]);

	const loggedInLinks: Route[] = [
		{ text: 'Home', href: '/', allowed: 'always' },
		{ text: 'Profile', href: '/profile', allowed: 'authenticated' }
	];

	const loggedOutLinks: Route[] = [
		{ text: 'Home', href: '/', allowed: 'always' },
		{ text: 'Register', href: '/register', allowed: 'unauthenticated' },
		{ text: 'Login', href: '/login', allowed: 'unauthenticated' }
	];

	function currentRoute(): Route | null {
		let matching = links.filter((v) => {
			return v.href == page.route.id;
		});
		return matching[0];
	}

	let title = $derived.by(() => {
		if (currentRoute()) {
			return currentRoute()?.text;
		}
		return 'Axumatic';
	});

	let loggedIn = $state(false);

	afterNavigate(async function () {
		let route = currentRoute();
		let response = await api.getProfile();
		if (response.response_type == 'Error') {
			loggedIn = false;
			links = loggedOutLinks;
			if (route?.allowed == 'authenticated') {
				goto('/login');
			}
		} else {
			loggedIn = true;
			links = loggedInLinks;
			if (route?.allowed == 'unauthenticated') {
				goto('/profile');
			}
		}
	});

	async function logout() {
		await api.logout();
		goto('/login');
	}
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
	<title>{title}</title>
</svelte:head>

<nav class="bg-blue-500 py-3">
	{#each links as link}
		<a class="mx-2 text-xl text-white hover:text-blue-100" href={link.href}>{link.text}</a>
	{/each}
	{#if loggedIn}
		<button class="mx-2 cursor-pointer text-xl text-white hover:text-blue-100" onclick={logout}
			>Logout</button
		>
	{/if}
</nav>
<main class="m-auto max-w-7xl px-2">
	{@render children?.()}
</main>

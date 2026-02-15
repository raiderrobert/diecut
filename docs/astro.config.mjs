// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

// https://astro.build/config
export default defineConfig({
	integrations: [
		starlight({
			title: 'diecut',
			logo: {
				src: './src/assets/logo.svg',
			},
			favicon: '/src/assets/favicon.png',
			social: [{ icon: 'github', label: 'GitHub', href: 'https://github.com/raiderrobert/diecut' }],
			sidebar: [
				{
					label: 'Getting Started',
					items: [
						{ label: 'Installation & Quick Start', slug: 'getting-started' },
					],
				},
				{
					label: 'Guides',
					items: [
						{ label: 'Using Templates', slug: 'using-templates' },
						{ label: 'Creating Templates', slug: 'creating-templates' },
					],
				},
				{
					label: 'Reference',
					items: [
						{ label: 'Commands', slug: 'reference/commands' },
						{ label: 'diecut.toml', slug: 'reference/diecut-toml' },
						{ label: 'Hooks', slug: 'reference/hooks' },
					],
				},
			],
		}),
	],
});

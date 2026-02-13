// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

// https://astro.build/config
export default defineConfig({
	integrations: [
		starlight({
			title: 'diecut',
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
						{ label: 'Migrating from Cookiecutter', slug: 'migrating-from-cookiecutter' },
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

import * as path from 'node:path';
import { defineConfig } from '@rspress/core';

export default defineConfig({
  root: path.join(__dirname, 'docs'),
  base: '/project-docs/',
  title: 'Project Documentation',
  description: 'Internal developer documentation',
  llms: true,
  themeConfig: {
    socialLinks: [],
  },
});

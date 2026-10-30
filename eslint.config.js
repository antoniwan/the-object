// The lints for the JavaScript: the page's own scripts (web/), the serverless function (api/) and
// the build and test scripts (scripts/). Formatting is Prettier's (.prettierrc.json), not this.
import js from '@eslint/js';
import globals from 'globals';

export default [
  { ignores: ['web/pkg/', 'target/', 'dist/', 'node_modules/'] },
  js.configs.recommended,
  {
    linterOptions: { reportUnusedDisableDirectives: 'error' },
    rules: {
      eqeqeq: ['error', 'always', { null: 'ignore' }],
      'no-empty': ['error', { allowEmptyCatch: true }],
      'no-var': 'error',
      'prefer-const': 'error',
      'no-unused-vars': ['error', { argsIgnorePattern: '^_', caughtErrors: 'none' }],
    },
  },
  { files: ['web/**/*.js'], languageOptions: { globals: globals.browser } },
  { files: ['api/**/*.js', 'scripts/**/*.{js,mjs}', '*.js'], languageOptions: { globals: globals.node } },
  // The scripts that drive a page with Playwright hand functions to it to run there.
  { files: ['scripts/**/*.mjs'], languageOptions: { globals: { ...globals.node, ...globals.browser } } },
];

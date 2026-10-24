import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vite-plus'

export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    target: 'esnext',
    minify: process.env.TAURI_DEBUG ? false : 'oxc',
    sourcemap: Boolean(process.env.TAURI_DEBUG),
  },
  staged: {
    '*': 'vp check --fix',
  },
  fmt: {
    singleQuote: true,
    semi: false,
    ignorePatterns: ['dist/**', 'test-results/**', 'playwright-report/**'],
  },
  lint: {
    plugins: ['typescript', 'vue'],
    // Everything is an error, as in the sibling Pili frontends: a warning would just pile up unnoticed.
    categories: {
      correctness: 'error',
      suspicious: 'error',
      pedantic: 'error',
      perf: 'error',
      style: 'error',
    },
    rules: {
      'no-shadow': 'error',
      'default-case-last': 'error',
      'prefer-const': 'error',
      'no-var': 'error',
      eqeqeq: 'error',
      curly: 'off',
      'id-length': 'off',
      'max-lines-per-function': 'off',
      'max-lines': 'off',
      'max-statements': 'off',
      'func-style': 'off',
      'no-magic-numbers': 'off',
      'no-ternary': 'off',
      'no-nested-ternary': 'off',
      'sort-keys': 'off',
      'no-inline-comments': 'off',
      'init-declarations': 'off',
      'no-continue': 'off',
      'no-await-in-loop': 'off',
      'unicorn/no-nested-ternary': 'off',
      'one-var': 'off',
      'vite-plus/prefer-vite-plus-imports': 'error',
      // The autofix capitalises the first letter of every comment line, which mangles continuation lines.
      'capitalized-comments': 'off',
      // Two autofixes the pre-commit hook (`vp check --fix`) would apply blindly.
      'typescript/no-unnecessary-type-assertion': 'off',
      'typescript/strict-void-return': 'off',
      // Wants every parameter deeply readonly, which Vue refs, DOM events and most library types can never be.
      'typescript/prefer-readonly-parameter-types': 'off',
      // Import order is the formatter's job.
      'sort-imports': 'off',
    },
    overrides: [
      {
        // Playwright specs and the WebDriver tests of the real binary drive a page from the outside: they cast mocks,
        // build partial objects and read untyped values from the page. These rules are noise there, and only there.
        files: ['e2e/**'],
        rules: {
          'typescript/no-unsafe-type-assertion': 'off',
          'typescript/no-unsafe-member-access': 'off',
          'typescript/no-unsafe-return': 'off',
          'typescript/no-unsafe-assignment': 'off',
          'typescript/no-unsafe-call': 'off',
          'typescript/no-unsafe-argument': 'off',
          'typescript/strict-boolean-expressions': 'off',
          'typescript/no-non-null-assertion': 'off',
          // Test doubles: classes with one-line constructors, `__`-named state the IPC mock keeps on the window,
          // throw-away regexes and function expressions that need their own `this`.
          'typescript/parameter-properties': 'off',
          'max-classes-per-file': 'off',
          'no-underscore-dangle': 'off',
          'max-params': 'off',
          'prefer-named-capture-group': 'off',
          'new-cap': 'off',
          'func-names': 'off',
        },
      },
    ],
    env: {
      browser: true,
    },
    ignorePatterns: ['dist', 'node_modules', 'test-results', 'playwright-report'],
    options: {
      typeAware: true,
      typeCheck: true,
    },
    jsPlugins: [
      {
        name: 'vite-plus',
        specifier: 'vite-plus/oxlint-plugin',
      },
    ],
  },
})

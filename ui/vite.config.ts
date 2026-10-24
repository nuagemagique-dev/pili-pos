import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vite-plus'

// `TAURI_DEBUG` is set (to any text) by `tauri dev` and `tauri build --debug`.
const debug = (process.env.TAURI_DEBUG ?? '') !== ''

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
    minify: debug ? false : 'oxc',
    sourcemap: debug,
  },
  // Unit tests only: the Playwright specs (e2e/*.spec.ts) and the real-binary tests (e2e/real-binary/*.test.mjs)
  // have their own runners.
  test: {
    include: ['src/**/*.test.ts', 'scripts/**/*.test.ts'],
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
        // A wrapper takes its Tauri command's arguments one by one, in the order of the engine function and the
        // Android FFI method it is checked against (scripts/android/check_command_parity.py).
        files: ['src/api/**'],
        rules: {
          'max-params': 'off',
        },
      },
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
          // Tauri's invoke rejects with the backend's refusal string, never an Error: the IPC mock and the handlers of
          // the specs reject and throw strings as the real commands do.
          'no-throw-literal': 'off',
          'typescript/only-throw-error': 'off',
          'prefer-promise-reject-errors': 'off',
          'typescript/prefer-promise-reject-errors': 'off',
        },
      },
      {
        // node:test registers each top-level `test()`; the runner owns the promise it returns.
        files: ['e2e/real-binary/**'],
        rules: {
          'typescript/no-floating-promises': 'off',
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

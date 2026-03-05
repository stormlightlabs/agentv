// @ts-check
import js from "@eslint/js";
import svelte from "eslint-plugin-svelte";
import globals from "globals";
import { defineConfig } from "eslint/config";
import ts from "typescript-eslint";
import svelteConfig from "./svelte.config.js";
import unicorn from "eslint-plugin-unicorn";

export default defineConfig(
  js.configs.recommended,
  ...ts.configs.recommended,
  ...svelte.configs.recommended,
  unicorn.configs.recommended,
  { languageOptions: { globals: { ...globals.browser, ...globals.node } } },
  {
    files: ["**/*.svelte", "**/*.svelte.ts", "**/*.svelte.js"],
    languageOptions: {
      parserOptions: {
        projectService: true,
        extraFileExtensions: [".svelte"],
        parser: ts.parser,
        svelteConfig: { ...svelteConfig, kit: { ...svelteConfig.kit, typescript: undefined } },
      },
    },
    rules: { "unicorn/filename-case": "off", "unicorn/no-null": "off", "unicorn/prevent-abbreviations": "off" },
  },
);

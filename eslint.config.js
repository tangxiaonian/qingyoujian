import ts from "typescript-eslint";
import vue from "eslint-plugin-vue";
import prettier from "eslint-config-prettier";

export default [
  ...ts.configs.recommended,
  ...vue.configs["flat/recommended"],
  {
    files: ["**/*.vue"],
    languageOptions: { parserOptions: { parser: ts.parser } },
  },
  prettier,
  { rules: { curly: ["error", "all"] } },
];

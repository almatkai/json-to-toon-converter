/// <reference types="@raycast/api">

/* 🚧 🚧 🚧
 * This file is auto-generated from the extension's manifest.
 * Do not modify manually. Instead, update the `package.json` file.
 * 🚧 🚧 🚧 */

/* eslint-disable @typescript-eslint/ban-types */

type ExtensionPreferences = {
  /** Default Action - Choose whether to copy the result or paste it to the active application by default. */
  "defaultAction": "copy" | "paste",
  /** Input Source - Choose whether to try selected text first or always use clipboard. */
  "inputSource": "auto" | "clipboard"
}

/** Preferences accessible in all the extension's commands */
declare type Preferences = ExtensionPreferences

declare namespace Preferences {
  /** Preferences accessible in the `convert-json` command */
  export type ConvertJson = ExtensionPreferences & {}
}

declare namespace Arguments {
  /** Arguments passed to the `convert-json` command */
  export type ConvertJson = {}
}


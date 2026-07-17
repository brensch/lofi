/// <reference types="vite/client" />

declare const __BUILD_REVISION__: string;

declare module "*.js?url" {
  const url: string;
  export default url;
}

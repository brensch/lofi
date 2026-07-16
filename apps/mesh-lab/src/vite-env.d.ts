/// <reference types="vite/client" />

declare module "*.js?url" {
  const url: string;
  export default url;
}

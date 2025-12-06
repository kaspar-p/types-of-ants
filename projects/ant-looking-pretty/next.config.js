/** @type {import('next').NextConfig} */
const nextConfig = {
  // Generates a NodeJS server that can be run like 'node server.js' and packages everything needed.
  // This allows a backend-for-frontend (BFF) architecture to be used instead of the
  // single-page app (SPA) pattern, which is usually a HUGE performance boost, while having all of
  // the dynamic advantages of pure backends.
  output: "standalone",

  trailingSlash: true,

  // See docs/debug/2025may03.md for a learning!
  // https://nextjs.org/docs/app/api-reference/config/next-config-js/generateBuildId
  generateBuildId: async () => {
    return process.env.commit_sha;
  },
};

module.exports = nextConfig;

/** @type {import('next').NextConfig} */
const nextConfig = {
  output: "export",
  trailingSlash: true,

  // See docs/debug/2025may03.md for a learning!
  // https://nextjs.org/docs/app/api-reference/config/next-config-js/generateBuildId
  generateBuildId: async () => {
    return process.env.commit_sha;
  },
};

module.exports = nextConfig;

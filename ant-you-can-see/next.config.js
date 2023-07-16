/** @type {import('next').NextConfig} */
const nextConfig = {
  output: "export",
  rewrites: () => [
    {
      source: "/:slug*",
      destination: "/:slug*.html",
    },
  ],
};

module.exports = nextConfig;

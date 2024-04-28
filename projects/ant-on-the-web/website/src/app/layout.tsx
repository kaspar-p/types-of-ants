"use client";

import "./globals.css";
import { Inter } from "next/font/google";
import React from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { Header } from "@/components/Header";

const inter = Inter({ subsets: ["latin"] });
const queryClient = new QueryClient();

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <QueryClientProvider client={queryClient}>
      <html lang="en">
        <body className={inter.className} style={{ fontFamily: "serif" }}>
          <Header>{children}</Header>
        </body>
      </html>
    </QueryClientProvider>
  );
}

"use client";

import "./globals.css";
import { Inter } from "next/font/google";
import { PropsWithChildren } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { Header } from "@/components/Header";
import Footer from "@/components/Footer";
import { UserProvider } from "@/components/UserProvider";

const inter = Inter({ subsets: ["latin"] });
const queryClient = new QueryClient();

export default function RootLayout({ children }: PropsWithChildren<{}>) {
  return (
    <QueryClientProvider client={queryClient}>
      <html lang="en">
        <UserProvider>
          <body
            className={inter.className + " flex flex-col h-screen m-0"}
            style={{ fontFamily: "serif" }}
          >
            <Header />
            <div className="mb-auto p-2">{children}</div>

            <Footer />
          </body>
        </UserProvider>
      </html>
    </QueryClientProvider>
  );
}

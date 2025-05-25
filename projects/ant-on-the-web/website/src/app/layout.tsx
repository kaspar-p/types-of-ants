"use client";

import "./globals.css";
import { Inter } from "next/font/google";
import React, { useEffect, useState } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { Header } from "@/components/Header";
import { getUser, getUserSchema } from "@/server/queries";
import { TUserContext, UserContext } from "@/state/userContext";

const inter = Inter({ subsets: ["latin"] });
const queryClient = new QueryClient();

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const [user, setUser] = useState<TUserContext>({ loggedIn: false });

  useEffect(() => {
    async function checkLoggedIn() {
      const res = await getUser();
      if (res.ok) {
        const user = getUserSchema.transformer(
          getUserSchema.schema.parse(await res.json())
        );
        setUser({
          loggedIn: true,
          user: user.user,
        });
      }
    }

    checkLoggedIn();
  }, []);

  return (
    <QueryClientProvider client={queryClient}>
      <html lang="en">
        <body className={inter.className} style={{ fontFamily: "serif" }}>
          <UserContext.Provider value={{ user, setUser }}>
            <Header />
            {children}
          </UserContext.Provider>
        </body>
      </html>
    </QueryClientProvider>
  );
}

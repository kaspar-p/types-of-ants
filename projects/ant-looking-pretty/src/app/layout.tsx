import "./globals.css";
import { Inter } from "next/font/google";
import { PropsWithChildren } from "react";
import { Header } from "@/components/Header";
import Footer from "@/components/Footer";
import { Providers } from "@/app/providers";
import { getAuth } from "@/state/user";
import { revalidatePath } from "next/cache";

const inter = Inter({ subsets: ["latin"] });

export default function RootLayout({ children }: PropsWithChildren<{}>) {
  const user = getAuth();
  async function resetUser() {
    "use server";
    revalidatePath("/");
  }

  return (
    <html lang="en">
      <body
        className={inter.className + " flex flex-col h-screen m-0"}
        style={{ fontFamily: "serif" }}
      >
        <Providers user={user} resetUser={resetUser}>
          <Header />
          <div className="mb-auto p-2">{children}</div>
          <Footer />
        </Providers>
      </body>
    </html>
  );
}

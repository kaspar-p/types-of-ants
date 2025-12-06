import Link from "next/link";

export default function Footer() {
  return (
    <div className="bg-slate-100">
      <div className="m-1 p-1 flex flex-row justify-between gap-4">
        <div className="flex flex-row justify-evenly gap-4">
          <span>typesofants.org since 2022</span>
          <Link href="/contact">contact</Link>
        </div>
        <div className="flex flex-row justify-evenly gap-4">
          <Link key={1} href="/terms/terms-of-service">
            terms of service
          </Link>
          <Link key={2} href="/terms/privacy-policy">
            privacy policy
          </Link>
        </div>
      </div>
    </div>
  );
}

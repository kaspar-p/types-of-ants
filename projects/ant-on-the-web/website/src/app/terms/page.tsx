import Link from "next/link";
import { TermSection, TermSectionTitle } from "./terms-of-service/page";

export default function TermsPage() {
  return (
    <div className="h-full flex flex-col items-center justify-center">
      <TermSection>
        <TermSectionTitle>terms & conditions</TermSectionTitle>
        <p>
          The following are documents for the privacy policy, terms of service,
          and SMS terms of service for typesofants.org
        </p>
        <ul>
          <li>
            privacy policy:{" "}
            <Link href="/terms/privacy-policy">/terms/privacy-policy</Link>
          </li>
          <li>
            terms of service:{" "}
            <Link href="/terms/terms-of-service">/terms/terms-of-service</Link>
          </li>
          <li>
            sms terms of service:{" "}
            <Link href="/terms/terms-of-service/sms">
              /terms/terms-of-service/sms
            </Link>
          </li>
        </ul>
      </TermSection>
    </div>
  );
}

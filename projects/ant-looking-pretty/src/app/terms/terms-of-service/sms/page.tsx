import Link from "next/link";
import { TermSection, TermSectionTitle } from "../page";

export default function SmsTermsOfServicePage() {
  return (
    <div className="h-full flex flex-col items-center justify-center">
      <TermSection>
        <TermSectionTitle>sms terms of service</TermSectionTitle>
        <p>last Updated: 2025-08-03</p>
        <p>
          By opting in to receive SMS messages from typesofants.org (“we,” “us,”
          or “our”), you agree to these SMS Terms of Service. These terms apply
          exclusively to SMS/text messaging and are separate from our main{" "}
          <Link href="/terms/terms-of-service">Terms of Service</Link>.
        </p>
      </TermSection>
      <TermSection>
        <TermSectionTitle>what you&apos;re signing up for</TermSectionTitle>
        <p>
          When you opt in to SMS communications from typesofants.org, you
          consent to receive recurring messages sent via automated technology to
          the mobile number you provided. These messages may include:
        </p>
        <ul>
          <li>Authentication messages for secure sign-in</li>
          <li>Product availability updates</li>
          <li>Order confirmations or delivery notices</li>
          <li>
            Marketing messages about promotions, giveaways, or special releases
          </li>
          <li>Operational alerts or customer service follow-ups</li>
        </ul>
        <p>Consent to receive these messages is not a condition of purchase.</p>
      </TermSection>
      <TermSection>
        <TermSectionTitle>how to opt in</TermSectionTitle>
        <p>
          You can opt in to SMS messaging by creating your account and choosing
          the &quot;phone&quot; method of two-factor authentication. This is
          consent for typesofants.org to send SMS messages to the phone number
          provided. Message frequency varies. Standard message and data rates
          may apply. Contact your carrier for details.
        </p>
      </TermSection>
      <TermSection>
        <TermSectionTitle>how to opt out</TermSectionTitle>
        <p>
          You may opt out by using any other form of two-factor authentication
          to securely sign in to typesofants.org&apos;s services. For example,
          email-based two-factor authentication is available to all users and no
          further SMS messages would be sent to the mobile number provided.
        </p>
        <p>
          Note that at least one two-factor authentication method must be
          available at all times to use.
        </p>
      </TermSection>
      <TermSection>
        <TermSectionTitle>changing your number</TermSectionTitle>
        <p>
          If you change or transfer your mobile number, you agree to unsubscribe
          from our SMS program. This helps ensure new owners of your former
          number are not sent unintended messages.
        </p>
      </TermSection>
      <TermSection>
        <TermSectionTitle>privacy & data</TermSectionTitle>
        <p>
          We respect your privacy. Your phone number and any data collected
          through SMS signups will not be sold or shared with third parties for
          marketing purposes. We may share information with service providers
          only to operate our messaging program. For full details, please review
          our <Link href="/terms/privacy-policy">Privacy Policy</Link>.
        </p>
      </TermSection>
      <TermSection>
        <TermSectionTitle>program availability & limitations</TermSectionTitle>
        <p>
          SMS services may not be available on all carriers or in all locations.
          Delivery of messages is subject to effective transmission by your
          mobile provider, and we are not responsible for delayed or undelivered
          messages.
        </p>
      </TermSection>
      <TermSection>
        <TermSectionTitle>changes to this privacy policy</TermSectionTitle>
        <p>
          We may update this Privacy Policy from time to time, including to
          reflect changes to our practices or for other operational, legal, or
          regulatory reasons. We will post the revised Privacy Policy on this
          website, update the &quot;Last updated&quot; date and provide notice
          as required by applicable law.
        </p>
      </TermSection>
      <TermSection>
        <TermSectionTitle>contact</TermSectionTitle>
        <p>
          Should you have any questions about our privacy practices or this
          Privacy Policy, or if you would like to exercise any of the rights
          available to you, please email us at ants@typesofants.org or
          kaspar@typesofants.org.
        </p>
      </TermSection>
    </div>
  );
}

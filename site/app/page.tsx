import { Downloads } from "@/components/downloads";
import { Features } from "@/components/features";
import { Hero } from "@/components/hero";
import { Modes } from "@/components/modes";
import { PrivacySection } from "@/components/privacy-section";
import { SettingsShowcase } from "@/components/settings-showcase";
import { Steps } from "@/components/steps";

export default function Home() {
  return (
    <main id="main">
      <Hero />
      <Steps />
      <Modes />
      <Features />
      <SettingsShowcase />
      <PrivacySection />
      <Downloads />
    </main>
  );
}

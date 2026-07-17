import { FlaskConical, SlidersHorizontal } from "lucide-react";

interface AppNavigationProps {
  active: "judge" | "lab";
}

export function AppNavigation({ active }: AppNavigationProps) {
  return (
    <nav className="page-tabs" aria-label="Product views">
      <a className={active === "lab" ? "active" : ""} href="/"><SlidersHorizontal size={13} /> Lab</a>
      <a className={active === "judge" ? "active" : ""} href="/judge"><FlaskConical size={13} /> Listening</a>
    </nav>
  );
}

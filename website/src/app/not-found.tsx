import Link from "next/link";

export default function NotFound() {
  return (
    <div className="os-grid flex min-h-screen items-center justify-center px-5">
      <div className="os-panel os-ticks max-w-[560px] p-7">
        <div className="mono mb-3 text-[11px] uppercase tracking-[.18em] text-[var(--os-muted)]">404 · no such workspace</div>
        <h1 className="text-[1.5rem] font-semibold tracking-[-.02em]">That workspace does not exist.</h1>
        <p className="mt-3 text-[13.5px] leading-[1.6] text-[var(--os-muted)]">
          Every workspace in this operating system is reachable from Mission Control or the ⌘K palette.
        </p>
        <Link href="/" className="mono mt-5 inline-block rounded-[8px] bg-[var(--os-accent)] px-3.5 py-2 text-[12px] font-medium text-[var(--os-on-accent)]">
          go to mission control
        </Link>
      </div>
    </div>
  );
}

import type { SVGProps } from "react";

export function CloudSyncIcon({
  size = 24,
  ...props
}: SVGProps<SVGSVGElement> & { size?: number }) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.7}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      {...props}
    >
      <path d="M6.3 16.5H5a4 4 0 1 1 1-7.87 6.5 6.5 0 0 1 12.4-1.4A4.5 4.5 0 0 1 19 16h-1.3" />
      <path d="M8 15a4 4 0 0 1 6.8-2.8L16 13.5M16 10.5v3h-3M16 17a4 4 0 0 1-6.8 2.8L8 18.5M8 21.5v-3h3" />
    </svg>
  );
}

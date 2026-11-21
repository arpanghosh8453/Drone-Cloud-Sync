import * as CheckboxPrimitive from "@radix-ui/react-checkbox";
import { Check, Minus } from "lucide-react";
import type * as React from "react";
export function Checkbox(
  props: React.ComponentProps<typeof CheckboxPrimitive.Root>,
) {
  return (
    <CheckboxPrimitive.Root className="checkbox" {...props}>
      <CheckboxPrimitive.Indicator>
        {props.checked === "indeterminate" ? (
          <Minus size={12} />
        ) : (
          <Check size={12} />
        )}
      </CheckboxPrimitive.Indicator>
    </CheckboxPrimitive.Root>
  );
}

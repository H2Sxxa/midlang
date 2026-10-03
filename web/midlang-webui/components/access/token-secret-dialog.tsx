import { CheckIcon, CopyIcon } from "@phosphor-icons/react"
import { useState } from "react"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { useToast } from "@/hooks/use-toast"
import type { CreatedToken } from "@/hooks/use-tokens"

type TokenSecretDialogProps = {
  readonly token: CreatedToken
  readonly onOpenChange: (open: boolean) => void
}

export function TokenSecretDialog({ token, onOpenChange }: TokenSecretDialogProps) {
  const toast = useToast()
  const [copied, setCopied] = useState(false)

  const handleCopy = async () => {
    await navigator.clipboard.writeText(token.token)
    setCopied(true)
    toast.success({ message: "Token copied", description: "Store it somewhere secure." })
  }

  return (
    <Dialog open onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Token “{token.name}”</DialogTitle>
          <DialogDescription>
            This secret is shown once. Copy it now; the server only keeps its hash.
          </DialogDescription>
        </DialogHeader>
        <div className="border bg-muted p-3 font-mono text-xs break-all">{token.token}</div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Close
          </Button>
          <Button onClick={() => void handleCopy()}>
            {copied ? <CheckIcon /> : <CopyIcon />}
            {copied ? "Copied" : "Copy token"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

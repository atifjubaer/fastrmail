export interface Contact {
  id: string;
  name: string;
  email: string;
  phone?: string;
  notes?: string;
}

const STORAGE_KEY = "fastrmail_contacts";

const DEFAULT_CONTACTS: Contact[] = [
  {
    id: "c-1",
    name: "FastrMail System",
    email: "system@fastrsoft.com",
    notes: "Core system notifications & daemon reports",
  },
  {
    id: "c-2",
    name: "Security Operations",
    email: "security@fastrsoft.com",
    notes: "SpamGuard, DNSBL, and TLS telemetry",
  },
  {
    id: "c-3",
    name: "DKIM Signer",
    email: "postmaster@fastrsoft.com",
    notes: "Cryptographic key manager",
  },
];

export function getContacts(): Contact[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      return JSON.parse(raw);
    }
  } catch {
    // fallback
  }
  return DEFAULT_CONTACTS;
}

export function saveContacts(contacts: Contact[]) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(contacts));
  } catch {
    // ignore
  }
}

export function addContact(contact: Omit<Contact, "id">): Contact {
  const contacts = getContacts();
  const newContact: Contact = {
    ...contact,
    id: `c-${Date.now()}`,
  };
  contacts.push(newContact);
  saveContacts(contacts);
  return newContact;
}

export function deleteContact(id: string) {
  const contacts = getContacts().filter((c) => c.id !== id);
  saveContacts(contacts);
}

export function searchContacts(query: string): Contact[] {
  const q = query.toLowerCase().trim();
  if (!q) return getContacts();
  return getContacts().filter(
    (c) =>
      c.name.toLowerCase().includes(q) ||
      c.email.toLowerCase().includes(q) ||
      (c.notes && c.notes.toLowerCase().includes(q))
  );
}

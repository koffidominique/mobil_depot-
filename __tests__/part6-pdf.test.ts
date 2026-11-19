/** PDF des reçus : structure minimale vérifiable sans appareil. */
import { buildReceiptPdf, type ReceiptPdfInput } from '../src/core/receiptPdf';

const input: ReceiptPdfInput = {
  depotName: 'Depot Marcory',
  receiptNumber: 'RC-42',
  dateLabel: '19 nov. 12:30',
  customer: 'Maquis Le Palmier',
  lines: [{ label: 'Beaufort 50 cl (CASE)', quantity: '1', amountFcfa: '7200' }],
  subtotalFcfa: '7200',
  discountFcfa: '0',
  totalFcfa: '7200',
  paidFcfa: '7200',
  creditFcfa: '0',
  packagingLines: ['Casiers attendus/recus: 1/0'],
  notes: ['Recu commercial - pas une facture normalisee.'],
};

describe('PDF de reçu', () => {
  test('produit un PDF 1.4 ASCII avec xref cohérent', () => {
    const pdf = buildReceiptPdf(input);
    expect(pdf.startsWith('%PDF-1.4')).toBe(true);
    expect(pdf.endsWith('%%EOF\n')).toBe(true);
    expect(/[^\x00-\x7F]/.test(pdf)).toBe(false);

    const xref = pdf.slice(pdf.indexOf('xref\n'));
    const startxref = Number(pdf.match(/startxref\n(\d+)/)?.[1]);
    expect(pdf.slice(startxref, startxref + 4)).toBe('xref');
    const offsets = [...xref.matchAll(/^(\d{10}) 00000 n $/gm)].map((match) => Number(match[1]));
    expect(offsets).toHaveLength(5);
    offsets.forEach((offset, index) => {
      expect(pdf.slice(offset, offset + 7)).toBe(`${index + 1} 0 obj`);
    });
  });

  test('échappe les parenthèses et supprime les accents', () => {
    const pdf = buildReceiptPdf({ ...input, customer: 'Maquis (Cote) \\ test' });
    expect(pdf).toContain('Maquis \\(Cote\\) \\\\ test');
    expect(pdf).not.toContain('é');
  });
});

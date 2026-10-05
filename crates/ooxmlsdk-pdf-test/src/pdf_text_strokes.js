// MuPDF's XML device omits stroke state for text. Read it from the device
// callback, keyed by the same painted font/GID/origin; never infer from Unicode.
var doc = Document.openDocument(scriptArgs[0]);
for (var i = 0; i < doc.countPages(); ++i) {
    var device = new Device({
        strokeText: function (text, stroke, ctm, colorspace, color, alpha) {
            var state = {
                width: stroke.getLineWidth(),
                cap: stroke.getLineCap(),
                join: stroke.getLineJoin(),
                miter: stroke.getMiterLimit(),
                phase: stroke.getDashPhase(),
                dashes: stroke.getDashPattern() || [],
                transform: ctm.slice(0, 4),
                alpha: alpha
            };
            text.walk({
                showGlyph: function (font, matrix, gid) {
                    if (gid < 0) return;
                    print(JSON.stringify({
                        page_number: i + 1,
                        font: font.getName(),
                        glyph_id: gid,
                        origin: [
                            ctm[0] * matrix[4] + ctm[2] * matrix[5] + ctm[4],
                            ctm[1] * matrix[4] + ctm[3] * matrix[5] + ctm[5]
                        ],
                        stroke: state
                    }));
                }
            });
        }
    });
    doc.loadPage(i).run(device, [1, 0, 0, 1, 0, 0]);
}

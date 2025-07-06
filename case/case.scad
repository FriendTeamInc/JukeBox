// JukeBox Case

/* [General settings] */
// Case size (width & height)
cS = 108;
// Case corner radius (rounded corners)
cR = 3;
// Case mounting hardware offset
cmO = 7.5;
// Case mounting hardware bolt size
cmB = 3.5;
// Case mounting hardware nut size (center to corner)
cmN = 3.3;
// Face count on rounded objects
$fn=32;
// Case connector hole size
ioHS = 10;
// Case connector hole offset
ioHO = cS-cmO-17.5;
// Case connector hole buffer
ioHB = 0.125;

/* [Case top settings] */
// Case top height
ctH = 8;
// Case top wall size
ctW = 2.5;
// Case top mounting plate size
ctM = 9;
// Case top mounting plate height
ctMH = 3.4;

/* [Case bottom settings] */
// Case bottom lip height
clH = 3;
// Case bottom lip chamfer size
clC = 1;
// Case bottom lip size
clS = 3;
// Case bottom PCB corner radius (rounded corners)
cpR = 3;
// Case bottom PCB height
cpH = 2.6;
// Case bottom PCB wall size
cpW = 3;
// Case bottom PCB mounting plate size
cpM = 9;
// Case bottom rubber feet spot offset
cpF = 20;
// Case bottom rubber feet spot diameter
cpFD = 10;
// Case bottom rubber feet spot depth
cpFH = 2;

/* [Logo settings] */
// Logo size scale
logoS = 0.035;
// Logo position (X)
logoX = 49;
// Logo position (Y)
logoY = 83;

/* [Keyboard settings] */
// Keyboard key size
kbS = 17;
// Keyboard key matrix position (X)
kbX = cS/2;
// Keyboard key matrix position (Y)
kbY = 39;
// Keyboard key matrix offset (X)
kbOX = 30;
// Keyboard key matrix offset (Y)
kbOY = 20;
// Keyboard key matrix width
kbW = 4;
// Keyboard key matrix height
kbH = 3;
// Keyboard key matrix spacing width
kbSW = 20;
// Keyboard key matrix spacing height
kbSH = 20;

/* [Case leg settings] */
// Leg width
clipW = 13;
// Leg rounding radius
clipR = 4;
// Leg Brace width
braceW = 20;

/* [Case screen settings] */
// Screen Cutout width
csScrCW = 57.5;
// Screen Cutout height
csScrCH = 36.5;
// Screen Window width
csScrWW = 42.5;
// Screen Window height
csScrWH = 32.5;

module rect4(x1, y1, x2, y2, h) {
    translate([x1, y1, h]) children();
    translate([x2, y1, h]) children();
    translate([x1, y2, h]) children();
    translate([x2, y2, h]) children();
}
module square4(s1, s2, h) {
    rect4(s1, s1, s2, s2, h) children();
}
// https://www.youtube.com/watch?v=gKOkJWiTgAY
module roundedsquare(xdim, ydim, zdim, rdim){
    hull() {
        rect4(rdim, rdim, xdim-rdim, ydim-rdim, 0) cylinder(h=zdim, r=rdim);
    }
}
module chamferedsquare(xdim, ydim, zdim, r1dim, r2dim){
    bigrdim = max(r1dim, r2dim);
    hull() {
        rect4(bigrdim, bigrdim, xdim-bigrdim, ydim-bigrdim, 0) cylinder(h=zdim, r1=r1dim, r2=r2dim);
    }
}

module speaker_icon() {
    scale([5/6, 5/6, 1]) {
        difference() {
            cylinder(r=7, h=1, center=true);
            cylinder(r=6, h=1, center=true);
        }
        cube([1, 13, 1], center=true);
        translate([ 4, 0, 0]) cube([1, 10, 1], center=true);
        translate([-4, 0, 0]) cube([1, 10, 1], center=true);
        translate([0, -2.25, 0]) cube([12, 1, 1], center=true);
        translate([0,  2.25, 0]) cube([12, 1, 1], center=true);
    }
}

SOX = cS / 2 - (csScrCW + ctW * 2) / 2; // screen origin x
SOY = cS - 18 - ctW - csScrCH / 2-2.75; // screen origin y

module case_top() {
    difference() {
        union() {
            difference() {
                union() {
                    // Top shell body
                    translate([0, 0, ctH]) chamferedsquare(cS, cS, 1, 3, 2);
                    roundedsquare(cS, cS, ctH, cR);
                }
                union() {
                    // USB-C hole
                    translate([-1, ioHO-ioHS, -1]) cube([ctW+2, ioHS, ctH+1]);
                    // Interior
                    translate([ctW, ctW, -1]) roundedsquare(cS-ctW*2, cS-ctW*2, ctH+1, cR);
                    // Screen cutout
                    translate([SOX, SOY, 0]) union() {
                        // Screen Window
                        translate([ctW+(csScrCW-csScrWW)/2, ctW+(csScrCH-csScrWH)/2, ctH]) cube([csScrWW, csScrWH, 1]);
                        // Inset
                        translate([ctW+(csScrCW+csScrWW)/2+2-52, 2.5, 0]) cube([52, csScrCH, 8.8]);
                    }

                    // // reset button cutout
                    // translate([cS-11, cS-25, ctH]) cylinder(h=1, d=6);

                    // identify led cutout
                    translate([cS-25, cS-10, ctH]) cylinder(h=1, d=2);
                }
            }

            h = ctH-ctMH;

            // Mounting plates
            square4(0, cS-ctM-ctW, h) roundedsquare(ctM+ctW, ctM+ctW, ctMH, cpR);
            translate([cmO, 0, h]) cube([cS - cmO*2, ctM+1.5, ctMH]);
            translate([0,          0, h]) roundedsquare(ctM+ctW+4, ctM+ctW+60, ctMH, cpR);
            translate([cS-ctM-ctW-4, 0, h]) roundedsquare(ctM+ctW+4, ctM+ctW+60, ctMH, cpR);
            translate([0, cS-ctM-ctW-12, h]) roundedsquare(ctM+ctW, ctM+ctW+12, ctMH, cpR);

            // Supports (screen)
            translate([SOX, SOY, 0]) union() {
                translate([ctW+(csScrCW+csScrWW)/2+2, ctW+(csScrCH+csScrWH)/2-.75, h]) cube([20, 3, ctMH]);
                translate([ctW-(csScrCW-csScrWW)/2-12.5, ctW+(csScrCH+csScrWH)/2-.75, h]) cube([20, 3, ctMH]);

                translate([ctW+(csScrCW-cS)/2, 1, h]) cube([cS, 1.5, ctMH]);
            }

            // Supports (keyboard)
            translate([0, 47.5, h]) cube([cS, 3, ctMH]);
            translate([0, 27.5, h]) cube([cS, 3, ctMH]);
        }

        union() {
            // keyboard key holes
            kX = kbX-kbOX;
            kY = kbY-kbOY;
            ch = ctH + 1;
            for (w=[0:kbW-1]) {
                for (h=[0:kbH-1]) {
                    translate([kX + kbSW * w, kY + kbSH * h, ch]) cube([kbS, kbS, 2], center=true);
                }
            }

            // mounting hardware holes
            square4(cmO, cS - cmO, ctH - 4) {
                cylinder(d=cmB, h=5);
                translate([0, 0, 2]) cylinder(d2=6, d1=2, h=3);
            }
            
            // Jukebox logo
            // case_detail();
        }
    }
}

module case_bottom() {
    difference() {
        union() {
            // Case floor
            chamferedsquare(cS, cS, clC, cR-clC, cR);
            translate([0, 0, clC]) roundedsquare(cS, cS, clH-clC, cR);

            // PCB table
            difference() {
                translate([    clS,     clS, clH]) roundedsquare(cS-clS*2, cS-clS*2, cpH, cpR);
                translate([clS+cpW, clS+cpW, clH]) cube([cS-clS*2-cpW*2, cS-clS*2-cpW*2, cpH]);
            }
            square4(clS, cS-clS*2-cpW*2, clH) roundedsquare(cpM, cpM, cpH, cpR);

            // USB-C pillar
            translate([0, ioHO-ioHS+ioHB, clH]) cube([1.5, ioHS-ioHB*2, 4]);
        }

        union() {
            cmO2 = cS-cmO;
            nH = 2.375;
            square4(cmO, cmO2, 0) cylinder(d=cmB, h=7);
            square4(cmO, cmO2, 0) cylinder($fn=6, r=cmN, h=nH);
            square4(cmO, cmO2, nH) cylinder($fn=6, r1=cmN, r2=cmB/2, h=1);

            // Indents for rubber feet
            square4(cpF, cS-cpF, 0) cylinder(d=cpFD, h=cpFH/2);
            square4(cpF, cS-cpF, cpFH/2) cylinder(d1=cpFD, d2=cpFD-2, h=cpFH/2);

            // cutout for through hole components
            translate([clS, ioHO-ioHS+ioHB, clH]) cube([10, ioHS-ioHB*2, cpH]);
        }
    }
}

module case_detail() {
    x1 = cS / 2 - 37;
    x2 = cS / 2 + 37;
    y = cS - 18;
    h = ctH + 0.75;
    // if (!gen_scr)
    //     translate([logoX, logoY, h])
    //         linear_extrude(height=0.5, center=true)
    //             scale([logoS, logoS, 0.5])
    //                 import(file="../assets/textlogo.svg", center=true);
    translate([x1, y, h]) scale([1.1, 1.1, 0.5]) speaker_icon();
    translate([x2, y, h]) scale([1.1, 1.1, 0.5]) speaker_icon();
}

module case_leg() {
    lH = clH+ctH+1.25;
    lS = cS-4;
    
    points = [
        [   0,       0, 0],
        [  lS,       0, 0],
        [lS/3, lS/1.75, 0]
    ];
    legX = points[2][0] - points[1][0];
    legY = points[2][1] - points[1][1];
    angle = atan2(legX, legY);
    length1 = sqrt(legX * legX + legY * legY);
    length2 = sqrt(points[2][0] * points[2][0] + points[2][1] * points[2][1]);

    difference() {
        union() {
            translate([lS + clipR, clipR, 0]) cylinder(h=clipW, r=clipR);
            translate([lS + clipR, clipR, 0]) cube([clipR, clipR + lH, clipW]);

            translate([clipR, 2 * clipR + lH, 0]) hull() {
                translate(points[0]) cylinder(h=clipW, r=clipR);
                translate(points[1]) cylinder(h=clipW, r=clipR);
                translate(points[2]) cylinder(h=clipW, r=clipR);
            }
        }
        union() {
            translate([lS, clipR, -1]) cube([clipR, lH, clipW+2]);

            translate([clipR, 2 * clipR + lH, -1]) linear_extrude(height=clipW+2) polygon(points=[[0,0],[lS,0],[lS/3, lS/1.75]]);

            // cut out for rubber feet
            rotate([0, 0, -angle]) translate([length1-1.6-1.5, -length2+1, 0]) cube([1.5, length1-2, clipW]);
        }
    }

    translate([cS-cpF, lH+clipR, clipW/2]) rotate([90, 0, 0]) cylinder(d1=cpFD, d2=cpFD-2, h=cpFH/2);
    translate([cpF, lH+clipR, clipW/2]) rotate([90, 0, 0]) cylinder(d1=cpFD, d2=cpFD-2, h=cpFH/2);
}

module case_leg_brace() {
    braceL = cS - cpF * 2 + clipW / 2 * 2;
    difference() {
        union() {
            translate([0, 0, clipR]) rotate([0, 90, 0]) cylinder(h=braceW, r=clipR);
            cube([braceW, braceL, clipR * 2]);
            translate([0, braceL, clipR]) rotate([0, 90, 0]) cylinder(h=braceW, r=clipR);
        }
        union() {
            translate([0, 0, clipR]) cube([braceW, clipW, clipR]);
            translate([0, braceL-clipW, clipR]) cube([braceW, clipW, clipR]);
        }
    }
}

module case_screen_spacer() {
    difference() {
        cube([52, 36.5, 2]);
        translate([0, 8, -1]) cube([26, 20.5, 4]);
    }
}

translate([-3, 3, 9]) rotate([0, 180, 0]) case_top();
translate([3, 3, 0]) case_bottom();
translate([-10, 15, 0]) rotate([0, 0, 180]) case_leg();
translate([10, 15, 0]) rotate([0, 0, 180]) scale([-1, 1, 1]) case_leg();
translate([clipR+3, cS+6+braceW, 0]) rotate([0, 0, -90]) case_leg_brace();
scale([-1, 1, 1]) translate([clipR+3, cS+6+braceW, 0]) rotate([0, 0, -90]) case_leg_brace();
rotate([0, 0, -90]) translate([25, -18.25, 0]) case_screen_spacer();

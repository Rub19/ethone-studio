#!/usr/bin/perl
# Collects every tl("…") string of the given sources into a FR<TAB>EN dictionary, keeping the
# translations already there. Untranslated entries get an empty English column.
# usage: perl scripts/i18n-extract.pl <dictionary.tsv> <files…>
use strict; use warnings;
my $dict = shift or die "usage: i18n-extract.pl <dict.tsv> <files>\n";
my (%en, @order, %seen);
my @head;
if (open my $d, '<:utf8', $dict) {
  while (my $l = <$d>) {
    chomp $l; $l =~ s/\r$//;
    if ($l =~ /^#/) { push @head, $l; next }
    my ($fr, $e) = split /\t/, $l, 2;
    next unless defined $fr && length $fr;
    $en{$fr} = $e // '';
    push @order, $fr unless $seen{$fr}++;
  }
}
my $new = 0;
for my $f (@ARGV) {
  open my $in, '<:utf8', $f or die "$f: $!"; local $/; my $s = <$in>; close $in;
  while ($s =~ /\btl\("((?:[^"\\]|\\.)*)"\)/g) {
    (my $fr = $1) =~ s/\\"/"/g;
    $fr =~ s/\\n/ /g;
    next if $seen{$fr}++;
    push @order, $fr; $en{$fr} = ''; $new++;
  }
}
@head = ("# Texte français<TAB>texte anglais. Une ligne par texte ; colonne anglaise vide = reste en français.") unless @head;
open my $out, '>:utf8', $dict or die; print $out join("\n", @head), "\n";
print $out "$_\t$en{$_}\n" for @order;
close $out;
my $missing = grep { $en{$_} eq '' } @order;
print "$dict: ", scalar(@order), " entries, $new new, $missing to translate\n";

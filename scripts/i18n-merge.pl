#!/usr/bin/perl
# Fills the empty English column of a dictionary from a FR<TAB>EN translations file.
# usage: perl scripts/i18n-merge.pl <dictionary.tsv> <translations.tsv>
use strict; use warnings;
my ($dict, $src) = @ARGV;
die "usage: i18n-merge.pl <dict.tsv> <translations.tsv>\n" unless $dict && $src;
my %tr;
open my $t, '<:utf8', $src or die "$src: $!";
while (my $l = <$t>) {
  chomp $l; $l =~ s/\r$//;
  my ($fr, $en) = split /\t/, $l, 2;
  $tr{$fr} = $en if defined $en && length $en && $fr !~ /^#/;
}
close $t;
open my $in, '<:utf8', $dict or die "$dict: $!"; my @l = <$in>; close $in;
my ($done, $left) = (0, 0);
for (@l) {
  next if /^#/;
  chomp(my $line = $_); $line =~ s/\r$//;
  my ($fr, $en) = split /\t/, $line, 2;
  next unless defined $fr && length $fr;
  if ((!defined $en || $en eq '') && exists $tr{$fr}) { $_ = "$fr\t$tr{$fr}\n"; $done++ }
  elsif (!defined $en || $en eq '') { $left++ }
}
open my $out, '>:utf8', $dict or die; print $out @l; close $out;
print "$dict: $done filled, $left still empty\n";

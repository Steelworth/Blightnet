#!/usr/bin/env python3
"""Write data/names.json — ~250 male, ~250 female given names, ~500 surnames."""
from __future__ import annotations

import json
from pathlib import Path

MALE = """
Aaron Abel Abraham Adam Adrian Aidan Alan Albert Alejandro Alex Alexander Alfred Ali
Allan Allen Alvin Amir Amos Andre Andrew Angel Anthony Antonio Archer Arlo Arthur Asher
Austin Avery Axel Barney Barry Beau Ben Benjamin Bennett Bernard Bert Bill Billy Blake
Bob Bobby Brad Bradley Brandon Brayden Brendan Brennan Brent Brett Brian Brock Brody
Bruce Bruno Bryan Byron Caleb Calvin Cameron Carl Carlos Carson Carter Casey Charles
Charlie Chase Chris Christian Christopher Clarence Clark Claude Clayton Clifford Clint
Clive Cody Cole Colin Connor Conrad Cooper Corey Craig Curtis Cyrus Dale Damian Damien
Damon Dan Daniel Danny Darren Darryl Dave David Dean Dennis Derek Desmond Dexter Diego
Dominic Donald Douglas Drew Duane Duncan Dustin Dwayne Dwight Dylan Earl Ed Eddie Edgar
Edmund Edward Edwin Eli Elias Elijah Elliot Ellis Elmer Elton Emmanuel Eric Erik Ernest
Ethan Eugene Evan Everett Ezekiel Ezra Felix Fernando Finn Fletcher Floyd Forrest Frank
Franklin Fred Frederick Gabriel Gael Garrett Gary Gavin Gene Geoffrey George Gerald
Gerard Gianni Gilbert Glen Glenn Gordon Graham Grant Greg Gregory Gus Gustavo Guy Hank
Harold Harry Harvey Hayden Hector Henry Herbert Herman Holden Howard Hubert Hugh Hugo
Hunter Ian Ibrahim Isaac Isaiah Ivan Jack Jackson Jacob Jake James Jamie Jared Jason
Jasper Javier Jay Jayden Jeff Jeffrey Jeremy Jerome Jerry Jesse Jim Jimmy Joe Joel Joey
John Johnny Jon Jonathan Jordan Jorge Jose Joseph Josh Joshua Josiah Juan Julian Julio
Julius Justin Kane Karl Keith Ken Kenneth Kenny Kevin Khalil Kirk Kurt Kyle Lance Landon
Larry Lawrence Lee Leo Leon Leonard Leroy Levi Lewis Liam Lloyd Logan Lorenzo Lou Louis
Lucas Luis Luke Malcolm Malik Marc Marco Marcus Mario Mark Marlon Marshall Martin Marvin
Mason Mateo Matt Matthew Maurice Max Maxwell Melvin Micah Michael Miguel Mike Miles Milton
Mitch Mitchell Mohamed Morgan Morris Muhammad Mustafa Nathan Nathaniel Ned Neil Nelson
Nicholas Nick Nicolas Nigel Noah Noel Nolan Norman Oliver Omar Oscar Owen Pablo Patrick
Paul Pedro Percy Perry Pete Peter Phil Philip Quentin Quincy Quinn Ralph Ramon Randall
Randy Raul Ray Raymond Reed Reid Ricardo Richard Rick Ricky Rico Riley Rob Robert Roberto
Robin Rocco Rod Rodney Roger Roland Roman Ron Ronald Ross Roy Ruben Russell Ryan Sam
Samuel Santiago Santos Saul Scott Sean Sebastian Sergio Seth Shane Shawn Sidney Simon
Spencer Stan Stanley Stefan Stephen Steve Steven Stewart Stuart Ted Terrance Terry
Theodore Thomas Tim Timothy Tobias Todd Tom Tommy Tony Travis Trent Trevor Troy Tyler
Tyrone Tyson Ulysses Uriel Victor Vincent Virgil Wade Walker Walter Warren Wayne Wesley
Will William Willie Wilson Wyatt Xavier Yahir Yasir Yusuf Zach Zachary Zack Zane Zander
""".split()

FEMALE = """
Abigail Ada Adeline Adriana Adrienne Aisha Alana Alexandra Alice Alicia Alison Allison
Alyssa Amanda Amber Amelia Amy Ana Anastasia Andrea Angela Angelica Anita Ann Anna Anne
Annette Annie April Aria Ariana Arlene Ashley Aspen Athena Aubrey Audrey Autumn Ava
Barbara Beatrice Belinda Bella Bernice Bertha Bessie Beth Betsy Betty Beverly Bianca
Bonnie Brenda Bridget Brittany Brooke Caitlin Caitlyn Camille Candace Cara Carla Carmen
Carol Carolina Caroline Carolyn Carrie Cassandra Catherine Cathy Cecilia Celeste Celia
Charlotte Chelsea Cheryl Chloe Christina Christine Cindy Claire Clara Claudia Colleen
Connie Cora Courtney Crystal Cynthia Daisy Dana Danielle Daphne Darlene Dawn Deanna Debbie
Deborah Debra Denise Desiree Diana Diane Dolores Donna Dora Doris Dorothy Edith Edna
Eileen Elaine Eleanor Elena Elisa Elise Eliza Elizabeth Ella Ellen Ellie Eloise Elsa
Elsie Emilia Emily Emma Erica Erin Esther Ethel Eva Eve Evelyn Faith Fallon Fanny Felicia
Fern Fiona Florence Frances Francesca Gabriela Gabrielle Gail Gayle Georgia Geraldine
Gina Ginger Gladys Gloria Grace Gracie Gretchen Gwen Gwendolyn Hailey Hannah Harley
Harmony Harriet Hattie Hazel Heather Heidi Helen Holly Hope Ida Ingrid Irene Iris Irma
Isabel Isabella Isabelle Ivy Jackie Jacqueline Jamie Jane Janet Janice Janie Jasmine
Jean Jeanette Jeanne Jenna Jennifer Jenny Jessica Jill Jillian Joan Joanna Joanne Jocelyn
Jodi Jodie Johanna Josephine Joy Joyce Juanita Judith Judy Julia Julianne Julie June
Justine Kara Karen Kari Karla Kate Katelyn Katherine Kathleen Kathryn Kathy Katie Katrina
Kay Kayla Kaylee Kelly Kelsey Kendall Kendra Kennedy Kerry Kimberly Kirsten Krista Kristen
Kristin Kristina Kristine Krystal Kyla Kylee Lacey Lana Lara Laura Laurel Lauren Laurie
Leah Leanne Lena Leona Leslie Leticia Lila Lilian Lillian Lily Linda Lindsay Lisa Lois
Lola Loretta Lori Lorraine Louise Lucia Lucille Lucy Lydia Lyla Lynn Mabel Mackenzie
Madeline Madison Mae Maggie Mandy Mara Marcia Margaret Margo Maria Mariah Marian Marianne
Marie Marilyn Marina Marion Marisa Marisol Marissa Marjorie Marlene Marsha Martha Mary
Maryann Matilda Maureen Maxine Maya Megan Meghan Melanie Melinda Melissa Melody Mercedes
Meredith Mia Michelle Mildred Millie Mindy Miranda Miriam Misty Molly Mona Monica Morgan
Muriel Myra Myrtle Nadia Nadine Nancy Naomi Natalie Natasha Nichole Nicole Nina Noelle
Nora Norma Olga Olivia Paige Pamela Patricia Patti Paula Pauline Pearl Peggy Penelope
Penny Phyllis Piper Priscilla Rachel Ramona Rebecca Regina Renee Rhonda Rita Roberta Robin
Robyn Rosa Rosalie Rose Rosemary Rosie Roxanne Ruby Ruth Sabrina Sadie Sally Samantha
Sandra Sandy Sara Sarah Savannah Selena Serena Shannon Sharon Sheila Shelley Sherri
Sherry Shirley Sienna Sierra Silvia Sonia Sophia Sophie Stacey Stacy Stella Stephanie
Sue Susan Suzanne Sylvia Tabitha Tamara Tami Tammy Tanya Tara Taylor Teresa Teri Terri
Terry Tess Thelma Theresa Tiffany Tina Toni Tonya Tracey Tracy Ursula Valerie Vanessa
Vera Veronica Vicki Victoria Violet Virginia Vivian Wanda Wendy Whitney Willow Winnie
Yolanda Yvonne Zoe Zoey
""".split()

LAST = """
Abbott Acevedo Acosta Adams Adkins Aguilar Aguirre Alexander Ali Allen Allison Alvarado
Alvarez Andersen Anderson Andrade Andrews Anthony Armstrong Arnold Arroyo Ashley Atkins
Atkinson Austin Avery Avila Ayala Ayers Bailey Baird Baker Baldwin Ball Ballard Banks
Barber Barker Barnes Barnett Barr Barrett Barron Barry Bartlett Barton Bass Bates Bauer
Bautista Baxter Bean Beard Beasley Beck Becker Bell Bender Bennett Benson Bentley Benton
Berg Berger Bernard Berry Best Bird Bishop Black Blackburn Blackwell Blair Blake Blanchard
Blanco Bland Blankenship Blevins Bolton Bond Bonilla Booker Boone Booth Bowen Bowers
Bowman Boyd Boyer Boyle Bradford Bradley Bradshaw Brady Branch Brandt Braun Bray Brennan
Brewer Bridges Briggs Bright Britt Brock Brooks Brown Browning Bruce Buchanan Buck Buckley
Buckner Bullock Burch Burgess Burke Burnett Burns Burton Bush Butler Byrd Cabrera Cain
Calderon Caldwell Calhoun Callahan Camacho Cameron Campbell Campos Cannon Cantrell Cantu
Cardenas Carey Carlson Carpenter Carr Carrillo Carroll Carson Carter Case Casey Castaneda
Castillo Castro Cervantes Chambers Chan Chandler Chaney Chang Chapman Charles Chase Chavez
Chen Cherry Choi Christensen Christian Chung Church Cisneros Clark Clarke Clay Clayton
Clements Cline Cobb Cochran Coffey Cohen Cole Coleman Collier Collins Colon Combs Compton
Conley Conner Conrad Contreras Conway Cook Cooper Copeland Corona Cortes Cortez Costa
Cowan Cox Craig Crane Crawford Crosby Cross Cruz Cuevas Cummings Cunningham Curry Curtis
Dalton Daniel Daniels Daugherty Davenport David Davidson Davies Davis Dawson Day Dean
Decker Dejesus Delacruz Delaney Deleon Delgado Dennis Diaz Dickerson Dixon Dodson Dominguez
Donaldson Donovan Dorsey Dougherty Douglas Downs Doyle Drake Duarte Dudley Duffy Duke
Duncan Dunlap Dunn Duran Durham Dyer Eaton Edwards Elliott Ellis Ellison English Erickson
Escobar Esparza Espinoza Estes Estrada Evans Everett Ewing Farley Farmer Farrell Faulkner
Ferguson Fernandez Ferrell Fields Figueroa Finch Finley Fischer Fisher Fitzgerald Fitzpatrick
Fleming Fletcher Flores Flowers Floyd Flynn Foley Ford Foster Fowler Fox Francis Franco
Frank Franklin Frazier Frederick Freeman French Frey Frost Fry Frye Fuentes Fuller Gaines
Gallagher Gallegos Galloway Galvan Gamble Garcia Gardner Garner Garrett Garrison Garza
Gates Gay Gentry George Gibbs Gibson Gilbert Giles Gill Gillespie Gilmore Glass Glenn
Glover Golden Gomez Gonzales Gonzalez Good Goodman Goodwin Gordon Gould Graham Grant Graves
Gray Green Greene Greer Gregory Griffin Griffith Grimes Gross Guerra Guerrero Gutierrez
Guzman Haas Hahn Hale Haley Hall Hamilton Hammond Hampton Hancock Haney Hanna Hansen
Hanson Hardin Harding Hardy Harmon Harper Harrell Harrington Harris Harrison Hart Hartman
Harvey Hatch Hawkins Hay Hayden Hayes Haynes Hays Heath Hebert Henderson Hendricks Hendrix
Henry Hensley Henson Herman Hernandez Herrera Herring Hess Hester Hickman Hicks Higgins
Hill Hines Hinton Ho Hobbs Hodge Hodges Hoffman Hogan Holden Holder Holland Holloway
Holmes Holt Hood Hooper Hoover Hopkins Horn Horne Horton House Houston Howard Howe Howell
Huang Hubbard Huber Hudson Huff Huffman Hughes Hull Humphrey Hunt Hunter Hurley Hurst
Hutchinson Huynh Ibarra Ingram Irwin Jackson Jacobs Jacobson James Jarvis Jefferson Jenkins
Jennings Jensen Jimenez Johns Johnson Johnston Jones Jordan Joseph Joyce Juarez Kane
Kaufman Keith Keller Kelley Kelly Kemp Kennedy Kent Kerr Key Khan Kidd Kim King Kirby
Kirk Klein Kline Knapp Knight Knox Koch Kramer Lam Lamb Lambert Landry Lane Lang Langley
Lara Larsen Larson Lawrence Lawson Le Leach Leblanc Lee Leon Leonard Lester Levy Lewis
Li Lim Lin Lindsey Little Liu Livingston Lloyd Logan Long Lopez Love Lowe Lowery Lozano
Lucas Lucero Lugo Luna Lynch Lynn Lyons Macdonald Macias Mack Madden Maddox Mahoney
Maldonado Malone Mann Manning Marks Marquez Marsh Marshall Martin Martinez Mason Massey
Mathews Mathis Matthews Maxwell May Mayer Maynard Mayo Mays McBride McCall McCarthy
McClain McClure McConnell McCormick McCoy McCray McCullough McDaniel McDonald McDowell
McFarland McGee McGuire McIntosh McIntyre McKay McKee McKenzie McKinney McKnight McLaughlin
McLean McMahon McMillan McNeil Meadows Medina Mejia Melendez Melton Mendez Mendoza Mercado
Mercer Merrill Merritt Meyer Meyers Meza Michael Middleton Miles Miller Mills Miranda
Mitchell Molina Monroe Montes Montgomery Montoya Moody Moon Mooney Moore Mora Morales
Moran Moreno Morgan Morris Morrison Morrow Morse Morton Moses Mosley Moss Mueller Mullen
Mullins Munoz Murillo Murphy Murray Myers Nash Navarro Neal Nelson Newman Newton Nguyen
Nichols Nicholson Nielsen Nieto Nixon Noble Nolan Norman Norris Norton Novak Nunez OBrien
Ochoa OConnell OConnor Odom ODonnell Oliver Olsen Olson Oneal Oneill Orozco Orr Ortega
Ortiz Osborn Osborne Owen Owens Pace Pacheco Padilla Page Palmer Park Parker Parks Parrish
Parsons Patel Patterson Patton Paul Payne Pearson Peck Pena Pennington Perez Perkins
Perry Peters Petersen Peterson Petty Phelps Phillips Pierce Pittman Pitts Pittman Pitt
Ponce Poole Pope Porter Potter Potts Powell Powers Pratt Preston Price Prince Pritchard
Proctor Pruitt Pugh Quinn Ramirez Ramos Ramsey Randall Randolph Rangel Rasmussen Ray
Raymond Reed Reese Reeves Reid Reyes Reynolds Rhodes Rice Rich Richard Richards Richardson
Richmond Riddle Riggs Riley Rios Rivas Rivera Rivers Roach Robbins Roberson Roberts
Robertson Robinson Robles Rocha Rodgers Rodriguez Rogers Rojas Rollins Roman Romero Rosa
Rosales Rosario Rose Ross Roth Rowe Rowland Roy Rubio Ruiz Rush Russell Russo Ryan Salas
Salazar Salinas Sampson Sanchez Sanders Sandoval Sanford Santana Santiago Santos Saunders
Savage Sawyer Schmidt Schneider Schroeder Schultz Schwartz Scott Sellers Serrano Sexton
Shaffer Shannon Sharp Shaw Shelton Shepard Shepherd Sheppard Sherman Shields Short Silva
Simmons Simon Simpson Sims Singh Singleton Skinner Sloan Small Smith Snow Snyder Solis
Solomon Soto Sparks Spears Spence Spencer Stafford Stanley Stanton Stark Steele Stein
Stephens Stephenson Stevens Stevenson Stewart Stokes Stone Stout Strickland Strong Stuart
Suarez Sullivan Summers Sutton Swanson Sweeney Sweet Tanner Tapia Tate Taylor Terrell
Terry Thomas Thompson Thornton Todd Torres Townsend Tran Travis Trevino Trujillo Tucker
Turner Tyler Tyson Underwood Valdez Valencia Valentine Valenzuela Vance Vang Vargas Vasquez
Vaughan Vaughn Vazquez Vega Velasquez Velazquez Velez Villa Villarreal Villanueva Vincent
Wade Wagner Walker Wall Wallace Waller Walls Walsh Walter Walters Walton Wang Ward Warner
Warren Washington Waters Watkins Watson Watts Weaver Webb Weber Webster Weeks Wei Weiss
Welch Wells Welsh Werner West Wheeler Whitaker White Whitehead Whitney Wiggins Wilcox
Wiley Wilkerson Wilkins Wilkinson Williams Williamson Willis Wilson Winters Wise Witt
Wolf Wolfe Wong Wood Woodard Woods Woodward Wooten Wright Wu Wyatt Wynn Yang Yates Yoder
York Young Yu Zamora Zapata Zarate Zimmerman Zuniga
""".split()


def uniq(rows: list[str]) -> list[str]:
    seen = set()
    out = []
    for n in rows:
        key = n.casefold()
        if key in seen:
            continue
        seen.add(key)
        out.append(n)
    return out


def main() -> None:
    male = uniq(MALE)[:250]
    female = uniq(FEMALE)[:250]
    last = uniq(LAST)[:500]
    root = Path(__file__).resolve().parents[1]
    dest = root / "data" / "names.json"
    payload = {"male": male, "female": female, "last": last}
    dest.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {dest} male={len(male)} female={len(female)} first={len(male)+len(female)} last={len(last)}")
    if len(male) < 240 or len(female) < 240 or len(last) < 480:
        raise SystemExit("name lists too short")


if __name__ == "__main__":
    main()
